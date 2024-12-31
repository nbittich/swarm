import axios from "axios";
import { useAuth } from "@swarm/auth/authContextHook";
import { Outlet, useNavigate } from "react-router-dom";

export const ForbiddenInterceptor = () => {
  const navigate = useNavigate();
  const { setToken } = useAuth();
  axios.interceptors.response.use(
    (response) => {
      return response;
    },
    (error) => {
      console.log(error.response.status);
      if (error.status === 401 || error.status === 403) {
        localStorage.removeItem("token");
        setToken(null);
        navigate("/login", { replace: true });
      }
      return Promise.reject(error);
    }
  );
  return <Outlet />;
}
