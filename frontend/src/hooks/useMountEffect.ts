import { useEffect } from "react";

// eslint-disable-next-line react-hooks/exhaustive-deps
export default (fun: React.EffectCallback) => useEffect(fun, []);
