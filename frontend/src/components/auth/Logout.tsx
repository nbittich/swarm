import { useAuth } from "@swarm/auth/authContextHook";
import { useEffect } from "react";

const Logout = () => {
  const { setToken } = useAuth();

  // use effect?
  const handleLogout = () => {
    setToken(null);
  };

  useEffect(handleLogout);
  return null;
};

export default Logout;
