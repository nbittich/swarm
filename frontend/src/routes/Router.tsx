import { Navigate, RouterProvider, createBrowserRouter } from "react-router-dom";
import { ProtectedRoute } from "./ProtectedRoute";
import { useAuth } from "@swarm/auth/authContextHook";
import Logout from "@swarm/components/auth/Logout";
import Login from "@swarm/components/auth/Login";
import NoMatch from "@swarm/components/NoMatch";
import Layout from "@swarm/components/layout/Layout";
import { ForbiddenInterceptor } from "./ForbiddenInterceptor";
import JobsTable from "@swarm/components/jobs/JobsTable";
import TasksTable from "@swarm/components/jobs/TasksTable";
import SubTasksTable from "@swarm/components/jobs/SubTasksTable";
import ScheduledJobsTable from "@swarm/components/jobs/ScheduledJobsTable";
import Sparql from "@swarm/components/sparql/Sparql";
import SearchContainer from "@swarm/components/search/SearchContainer";

const SwarmRoutes = () => {
    const { token } = useAuth();

    const routesForPublic = [
        {
            path: "yasgui",
            element: <Sparql />,
        }, {
            path: "search",
            element: <SearchContainer />,
        },
        {
            path: "*",
            element: <NoMatch />
        }
    ];


    const routesForUserRole = [
        {
            path: "/",
            element: <ProtectedRoute />, // Wrap the component in ProtectedRoute
            children: [
                {
                    path: "",
                    element: <Navigate to="/jobs" />,
                },
                {
                    path: "jobs",
                    children: [
                        {
                            path: "",
                            element: <JobsTable />,
                        }
                        ,
                        {
                            path: ":id/tasks",
                            children: [
                                {
                                    path: "",
                                    element: <TasksTable />
                                },
                                {
                                    path: ":taskId/:taskName",
                                    element: <SubTasksTable />
                                }
                            ]
                        },
                    ]
                },
                {
                    path: "scheduled-jobs",
                    element: <ScheduledJobsTable />,
                },

                {
                    path: "logout",
                    element: <Logout />,
                },
            ],
        }
    ];


    const routesForNotAuthenticatedOnly = [

        {
            path: "/login",
            element: <Login />,
        }
    ];

    const router = createBrowserRouter([
        {
            element: <ForbiddenInterceptor />,
            children: [
                {
                    element: <Layout />,
                    children: [
                        ...routesForPublic,
                        ...(!token ? routesForNotAuthenticatedOnly : []),
                        ...routesForUserRole,

                    ]
                }
            ]
        }
    ]
    );

    return <RouterProvider router={router} />;
};

export default SwarmRoutes;
