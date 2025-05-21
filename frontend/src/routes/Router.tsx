import { Navigate, RouterProvider, createBrowserRouter } from "react-router-dom";
import { ProtectedRoute } from "./ProtectedRoute";
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
import Home from "@swarm/components/pages/Home";

const SwarmRoutes = () => {
    const routesForPublic = [
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
            path: "yasgui",
            element: <Sparql />,
        }, {
            path: "search",
            element: <SearchContainer />,
        },
        {
            path: "scheduled-jobs",
            element: <ScheduledJobsTable />,
        },
        {
            path: "/about",
            element: <Home />,
        },
        {
            path: "/",
            element: <Navigate to="/search" />
        },
        {
            path: "/login",
            element: <Login />,
        },
        {
            path: "*",
            element: <NoMatch />
        }
    ];


    const routesForUserRole = [
        {
            path: "/logout",
            element: <ProtectedRoute />, // Wrap the component in ProtectedRoute
            children: [
                {
                    path: "",
                    element: <Logout />,
                },
            ],
        }
    ];




    const router = createBrowserRouter([
        {
            element: <ForbiddenInterceptor />,
            children: [
                {
                    element: <Layout />,
                    children: [
                        ...routesForUserRole,

                        ...routesForPublic,
                        // ...(!token ? routesForNotAuthenticatedOnly : []),

                    ]
                }
            ]
        }
    ]
    );

    return <RouterProvider router={router} />;
};

export default SwarmRoutes;
