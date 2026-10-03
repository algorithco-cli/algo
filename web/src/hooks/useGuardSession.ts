import * as React from "react";
import { accountLogout } from "../lib/accountAuth";
import {
  GUARD_SESSION_EVENT,
  isAccountMode,
  loadStoredOrgId,
  loadStoredToken,
  saveStoredOrgId,
  saveStoredToken,
} from "../lib/billing";
import { sessionProfileFromToken } from "../lib/guard";

function snapshot(): { token: string; orgId: string } {
  const token = loadStoredToken();
  const accountMode = isAccountMode();
  const profile = sessionProfileFromToken(token, accountMode);
  const expired = profile?.expiresAt
    ? profile.expiresAt <= Math.floor(Date.now() / 1000)
    : false;
  return {
    token: profile && !expired ? token : "",
    orgId: loadStoredOrgId(),
  };
}

/** Reactive same-tab/cross-tab view of the locally stored backend session. */
export function useGuardSession() {
  const [state, setState] = React.useState(snapshot);

  React.useEffect(() => {
    const sync = () => setState(snapshot());
    window.addEventListener(GUARD_SESSION_EVENT, sync);
    window.addEventListener("storage", sync);
    return () => {
      window.removeEventListener(GUARD_SESSION_EVENT, sync);
      window.removeEventListener("storage", sync);
    };
  }, []);

  const selectOrg = React.useCallback((orgId: string) => {
    saveStoredOrgId(orgId);
  }, []);
  const logout = React.useCallback(() => {
    saveStoredOrgId("");
    if (isAccountMode()) {
      void accountLogout().catch(() => {
        // Local credentials are cleared before discovery; a remote logout
        // failure must never restore the session.
      });
      return;
    }
    saveStoredToken("");
  }, []);

  return {
    ...state,
    profile: sessionProfileFromToken(state.token, isAccountMode()),
    signedIn: state.token !== "",
    selectOrg,
    logout,
  };
}
