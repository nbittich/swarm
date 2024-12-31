import { Navigate, Outlet } from "react-router-dom";
import { useAuth } from "@swarm/auth/authContextHook";
import { App } from "antd";
export const ProtectedRoute = () => {
  const { token, } = useAuth();

  if (!token) {
    return <Navigate to="/login" />;
  }
  else {
    return <App><Outlet /></App>;
  }

};
