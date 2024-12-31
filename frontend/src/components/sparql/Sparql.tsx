import { useEffect, useRef, useState } from 'react';

const endpointUrl = import.meta.env.SPARQL_ENDPOINT;

const Sparql = () => {
  const containerRef = useRef(null);
  const [initialized, setInitialized] = useState(false);
  useEffect(() => {
    if (containerRef.current && !initialized) {
      Promise.all([
        import('@triply/yasgui'),
        import('@triply/yasgui/build/yasgui.min.css'),
        import('./YasguiCssOverride.css'),
      ]).then(([module]) => {
        const Yasgui = module.default;
        new Yasgui(containerRef.current!, {
          requestConfig: { endpoint: endpointUrl },
          autofocus: true,
        });
      });
      setInitialized(true);
    }
  }, [initialized]);

  return <>
    <h2>SPARQL</h2>
    <div ref={containerRef} style={{ height: '500px', width: '100%' }} />
  </>;
};

export default Sparql;
