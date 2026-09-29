import * as React from "react";
import {
  GUARD_SESSION_EVENT,
  loadStoredOrgId,
  loadStoredToken,
  saveStoredOrgId,
  saveStoredToken,
} from "../lib/billing";
import { sessionProfileFromToken } from "../lib/guard";

function snapshot(): { token: string; orgId: string } {
  const token = loadStoredToken();
  const profile = sessionProfileFromToken(token);
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
    saveStoredToken("");
    saveStoredOrgId("");
  }, []);

  return {
    ...state,
    profile: sessionProfileFromToken(state.token),
    signedIn: state.token !== "",
    selectOrg,
    logout,
  };
}
