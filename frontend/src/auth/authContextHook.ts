import { createContext, useContext } from "react";
import { UserClaims } from "@swarm/models/domain";

export interface AuthContextType {
  token: string | null;
  userClaims: UserClaims | null;
  setToken: (newToken: string | null) => void;
}
export const AuthContext = createContext<AuthContextType | undefined>(undefined);

export const useAuth = (): AuthContextType => {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return context;
};

