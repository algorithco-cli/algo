"""Jev probe/measurement HTTP client (Phase 0 verification).

Built ONLY from the public TypeSafe HTTP spec — https://docs.typesafe.ai/api
and https://docs.typesafe.ai/models (both verified 2026-09-20). There is NO
official Go/Rust SDK (https://docs.typesafe.ai/sdk, verified 2026-09-20), so
this client (and later the Rust agent client) speaks the documented HTTP
contract directly. No field outside the spec is sent or assumed.

Contract implemented:
  POST {base}/v1/systemone  Authorization: Bearer <key>  Content-Type: application/json
  body: {"state": str|obj|array, "model": str, "questions": {id: Question}}
  ok:   {"model": str, "answers": {id: Answer}, "usage": {"input_tokens": n, "output_tokens": n}}
  err:  401 / 422 / 429 / 529  (see docs/verify/jev-api.md §1)

Invariants:
  * Key comes from env ONLY (ALGO_JEV_API_KEY). Never a file, never argv, never logged.
  * Pre-send redaction hook runs on every state (canonical harness.redact when
    P0-EVAL-4 lands, local redaction.py stub until then).
  * Persistent HTTP/2 connection (httpx, http2=True), opened once per client.
  * Timeout default 800 ms (= L3 p99 budget) -> JevUnavailable -> caller maps to ASK.
  * Fail-safe: transport / timeout / parse / 429-exhausted / 529-exhausted errors
    ALL map to ask, never allow. Nothing here returns "allow" on an error path.
  * Logs carry hashes + counts only. The key and payloads are NEVER logged.
"""

from __future__ import annotations

import hashlib
import logging
import os
import time
from dataclasses import dataclass, field

import httpx

try:  # canonical harness redaction once P0-EVAL-4 lands
    from harness.redact import redact_text as _canonical_redact  # type: ignore
except Exception:  # pragma: no cover - stub path until harness exists
    _canonical_redact = None  # type: ignore

from redaction import log_record, redact_text as _stub_redact

log = logging.getLogger("jev_client")

BASE_URL_DEFAULT = "https://api.typesafe.ai"
API_PATH = "/v1/systemone"
MODELS_PATH = "/v1/models"
TIMEOUT_S_DEFAULT = 0.8  # L3 p99 budget; timeout -> ask (fail-safe)
MAX_RETRIES_DEFAULT = 2  # retries on 429/529 only, inside the timeout budget each


class JevError(Exception):
    """Base. Caller maps EVERY JevError to ask."""


class JevConfigError(JevError):
    """Missing key / bad config. Fix setup, do not retry traffic."""


class JevAuthError(JevError):
    """HTTP 401. Fix key, do not retry blindly."""


class JevValidationError(JevError):
    """HTTP 422. Our request is malformed — bug, log shape only."""


class JevUnavailable(JevError):
    """Timeout, network error, 429/529 exhausted, or bad payload. -> ask."""


def _redact(state: str) -> tuple[str, dict]:
    """Pre-send redaction hook. Returns (redacted_state, log_safe_record)."""
    if _canonical_redact is not None:
        redacted, count = _canonical_redact(state)
        stub = False
    else:
        redacted, count = _stub_redact(state)
        stub = True
    rec = log_record(redacted_state=redacted, substitutions=count)
    rec["stub"] = stub
    return redacted, rec


# --- typed question builders (spec §"Question types") -------------------------

def noul(instructions: str, true: str | None = None, false: str | None = None) -> dict:
    q: dict = {"type": "noul", "instructions": instructions}
    criteria = {}
    if true:
        criteria["true"] = true
    if false:
        criteria["false"] = false
    if criteria:
        q["criteria"] = criteria
    return q


def choice(instructions: str, criteria: dict[str, str | None]) -> dict:
    if not (2 <= len(criteria) <= 255):
        raise ValueError("choice needs 2..255 options per spec")
    return {"type": "choice", "instructions": instructions, "criteria": criteria}


def score(instructions: str, criteria: list[str]) -> dict:
    if not (2 <= len(criteria) <= 10):
        raise ValueError("score needs 2..10 levels per spec")
    return {"type": "score", "instructions": instructions, "criteria": criteria}


@dataclass
class Evaluation:
    action: str  # allow | ask | deny  (provisional mapping, see questions_version)
    confidence: float
    model: str  # versioned id echoed by the server — log per request
    usage: dict
    latency_ms: float
    questions_version: str
    answers_raw_types: dict = field(default_factory=dict)  # {qid: answer.type}, no payloads


def _map_to_action(answers: dict, questions_version: str) -> tuple[str, float]:
    """Provisional decision mapping (P0-EVAL-6 pins `questions-v0.1` properly).

    Convention: the question set MUST contain id "decision", a Choice over
    exactly {allow, ask, deny}. Action = argmax option; confidence = answer
    confidence. Any deviation (missing id, unknown option, unparsable shape)
    raises JevUnavailable -> ask. This keeps the error path fail-safe by
    construction: mapping bugs can only produce ask, never allow.
    """
    ans = answers.get("decision")
    if not isinstance(ans, dict) or ans.get("type") != "choice":
        raise JevUnavailable(f"missing/invalid 'decision' choice answer (set {questions_version})")
    opt = ans.get("choice")
    if opt not in ("allow", "ask", "deny"):
        raise JevUnavailable(f"unknown decision option: {opt!r}")
    try:
        conf = float(ans.get("confidence", 0.0))
    except (TypeError, ValueError):
        raise JevUnavailable("unparsable confidence")
    return opt, conf


class JevClient:
    """Persistent-HTTP/2 Jev client. Use as context manager (one connection)."""

    def __init__(
        self,
        *,
        api_key: str | None = None,
        base_url: str | None = None,
        model: str | None = None,
        timeout_s: float = TIMEOUT_S_DEFAULT,
        max_retries: int = MAX_RETRIES_DEFAULT,
    ) -> None:
        key = api_key or os.environ.get("ALGO_JEV_API_KEY", "")
        if not key:
            raise JevConfigError("ALGO_JEV_API_KEY is empty or unset (env-only; see .env.example)")
        self._key = key
        self.base_url = (base_url or os.environ.get("ALGO_JEV_BASE_URL", BASE_URL_DEFAULT)).rstrip("/")
        self.model = model or os.environ.get("ALGO_JEV_MODEL", "jev-latest")
        self.timeout_s = timeout_s
        self.max_retries = max_retries
        self._http = httpx.Client(
            http2=True,  # persistent multiplexed connection per plan §4.1
            timeout=httpx.Timeout(timeout_s),
            headers={"Authorization": "Bearer REDACTED", "Content-Type": "application/json"},
        )
        # The real key is injected per-request from memory (never stored in headers
        # visible to dumps, never logged). httpx keeps the TCP+TLS+HTTP/2 session.

    def close(self) -> None:
        self._http.close()

    def __enter__(self) -> "JevClient":
        return self

    def __exit__(self, *exc: object) -> None:
        self.close()

    # -- low level ---------------------------------------------------------

    def _post(self, payload: dict, *, red_log: dict) -> dict:
        headers = {"Authorization": f"Bearer {self._key}"}
        delays = [0.2, 0.4]
        attempt = 0
        while True:
            t0 = time.perf_counter()
            try:
                resp = self._http.post(self.base_url + API_PATH, json=payload, headers=headers)
            except (httpx.TimeoutException, httpx.TransportError) as e:
                # Timeout (default 800ms) and any transport error -> ask path.
                log.warning("jev transport/timeout -> ask", extra={"safe": {**red_log, "err": type(e).__name__}})
                raise JevUnavailable(f"transport/timeout: {type(e).__name__}") from e
            dt_ms = (time.perf_counter() - t0) * 1000.0
            if resp.status_code == 200:
                try:
                    data = resp.json()
                except ValueError as e:
                    raise JevUnavailable("unparsable 200 body") from e
                if not isinstance(data, dict) or "answers" not in data or "model" not in data:
                    raise JevUnavailable("200 body outside spec shape")
                data["_latency_ms"] = dt_ms
                return data
            if resp.status_code == 401:
                raise JevAuthError("401: missing/invalid key (check ALGO_JEV_API_KEY)")
            if resp.status_code == 422:
                raise JevValidationError("422: request failed validation (client bug)")
            if resp.status_code in (429, 529) and attempt < self.max_retries:
                wait = delays[min(attempt, len(delays) - 1)]
                retry_after = resp.headers.get("retry-after")
                try:
                    wait = max(wait, float(retry_after)) if retry_after else wait
                except ValueError:
                    pass
                log.info("jev %s -> backoff %.1fs (attempt %d)", resp.status_code, wait, attempt + 1)
                time.sleep(wait)
                attempt += 1
                continue
            if resp.status_code in (429, 529):
                raise JevUnavailable(f"{resp.status_code} exhausted after {attempt} retries")
            raise JevUnavailable(f"unexpected status {resp.status_code}")

    # -- public ------------------------------------------------------------

    def list_models(self) -> list[dict]:
        """GET /v1/models (spec). 1 probe request. Errors -> JevUnavailable."""
        try:
            resp = self._http.get(
                self.base_url + MODELS_PATH,
                headers={"Authorization": f"Bearer {self._key}"},
            )
        except (httpx.TimeoutException, httpx.TransportError) as e:
            raise JevUnavailable(f"models transport/timeout: {type(e).__name__}") from e
        if resp.status_code == 401:
            raise JevAuthError("401 on /v1/models")
        if resp.status_code != 200:
            raise JevUnavailable(f"models status {resp.status_code}")
        try:
            data = resp.json()
        except ValueError as e:
            raise JevUnavailable("unparsable models body") from e
        models = data.get("models", [])
        log.info("jev models listed", extra={"safe": {"count": len(models)}})
        return models

    def evaluate(self, *, state: str, questions: dict, questions_version: str) -> Evaluation:
        """Redact -> POST -> map to action. Any failure raises JevError (-> ask)."""
        redacted, red_log = _redact(state)
        payload = {"state": redacted, "model": self.model, "questions": questions}
        data = self._post(payload, red_log=red_log)
        action, conf = _map_to_action(data["answers"], questions_version)
        usage = data.get("usage", {})
        log.info(
            "jev decision",
            extra={"safe": {
                **red_log,
                "model": data.get("model"),
                "action": action,
                "confidence": round(conf, 4),
                "latency_ms": round(float(data["_latency_ms"]), 1),
                "usage": usage,
                "questions_version": questions_version,
                # NOTE: raw answer payloads are intentionally NOT logged.
                "answer_types": {k: (v.get("type") if isinstance(v, dict) else "?")
                                 for k, v in data["answers"].items()},
            }},
        )
        return Evaluation(
            action=action,
            confidence=conf,
            model=str(data.get("model")),
            usage=usage if isinstance(usage, dict) else {},
            latency_ms=float(data["_latency_ms"]),
            questions_version=questions_version,
            answers_raw_types={k: (v.get("type") if isinstance(v, dict) else "?")
                               for k, v in data["answers"].items()},
        )


def state_digest(redacted_state: str) -> str:
    """Hash for artifact dedup. Never reversible, safe to store."""
    return hashlib.sha256(redacted_state.encode("utf-8")).hexdigest()
