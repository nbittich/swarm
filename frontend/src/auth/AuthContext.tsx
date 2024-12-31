import axios from "axios";
import React, { useMemo, useState, ReactNode } from "react";
import { AuthContext } from "./authContextHook";
import { UserClaims } from "@swarm/models/domain";

interface AuthProviderProps {
  children: ReactNode;
}

const parseToken = (token: string | null): UserClaims | null => {
  try {
    if (token === null) {
      throw "token cannot be null";
    }
    return JSON.parse(atob(token.split('.')[1]));
  } catch (err) {
    console.warn("could not parse token:", err);
    return null;
  }
};

const AuthProvider: React.FC<AuthProviderProps> = ({ children }) => {
  const [token, _setToken] = useState<string | null>(localStorage.getItem("token"));
  const [userClaims, _setUserClaims] = useState<UserClaims | null>(parseToken(token));

  // Function to set the authentication token
  const setToken = (newToken: string | null) => {
    _setToken(newToken);
    _setUserClaims(parseToken(newToken));
  };

  if (token) {
    axios.defaults.headers.common["Authorization"] = "Bearer " + token;
    localStorage.setItem("token", token);
  } else {
    delete axios.defaults.headers.common["Authorization"];
    localStorage.removeItem("token");
  }

  const contextValue = useMemo(
    () => ({
      token,
      userClaims,
      setToken,
    }),
    [token, userClaims]
  );

  return (
    <AuthContext.Provider value={contextValue}>
      {children}
    </AuthContext.Provider>
  );
};


export default AuthProvider;
