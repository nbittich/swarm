import { useAuth } from '@swarm/auth/authContextHook';
import { RootState } from '@swarm/states/Store';
import { useEffect, useRef, useState } from 'react';
import { useSelector } from 'react-redux';

const endpointUrl = import.meta.env.VITE_SPARQL_ENDPOINT;
const defaultGraph = import.meta.env.VITE_DEFAULT_GRAPH;
const Sparql = () => {
    const { token } = useAuth();
    const containerRef = useRef(null);
    const darkMode = useSelector((state: RootState) => state.theme.darkMode);

    const [initialized, setInitialized] = useState(false);
    useEffect(() => {
        if (containerRef.current && !initialized) {
            Promise.all([
                import('@triply/yasgui'),
                import('@triply/yasgui/build/yasgui.min.css'),
                import('./YasguiCssDark.css'),
                import('./YasguiCssDefault.css'),
            ]).then(([module]) => {
                const Yasgui = module.default;

                new Yasgui(containerRef.current!, {
                    requestConfig: {
                        method: "POST",
                        endpoint: endpointUrl,
                        defaultGraphs: [defaultGraph],
                        headers: () => ({
                            Authorization: `Bearer ${token}`,
                        })
                    },

                    autofocus: true,

                });

            });
            setInitialized(true);
        }
    }, [initialized, token, darkMode]);

    return <>
        <h2>SPARQL</h2>
        <div className={darkMode ? "dark" : "light"} ref={containerRef} style={{ height: '500px', width: '100%' }} />
    </>;
};

export default Sparql;
