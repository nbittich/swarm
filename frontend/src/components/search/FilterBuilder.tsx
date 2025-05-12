import { useIsMobile } from "@swarm/hooks/is-mobile";
import { IndexConfiguration } from "@swarm/models/domain";
import { Button, Flex, Input, Select, } from "antd";

interface FilterBuilderProps {
    indexConfig: IndexConfiguration;
    conditions: {
        key: string;
        operator: string;
        joiner: string;
        value: string;
    }[];
    setConditions: (cond: {
        key: string;
        operator: string;
        joiner: string;
        value: string;
    }[]) => void;
}

const FilterBuilder: React.FC<FilterBuilderProps> = ({ indexConfig, conditions, setConditions }) => {

    const isMobile = useIsMobile();
    return (<>
        {conditions.map((cond, idx) => (
            <Flex vertical={isMobile} key={idx} gap="middle" style={{ paddingTop: '5px' }} >
                <Select
                    showSearch
                    placeholder="Field"
                    style={{ width: "100%" }}
                    onChange={val => {
                        const newConditions = [...conditions];
                        newConditions[idx] = { ...newConditions[idx], key: val };
                        setConditions(newConditions);
                    }}
                    value={cond.key}
                    options={indexConfig.properties.map(prop => ({ value: prop.name, label: prop.name }))}
                />
                <Select
                    style={{ width: "100%" }}
                    options={[{ value: '=', label: '=' }, { value: '!=', label: '!=' },
                    // { value: '>', label: '>' },
                    // { value: '>=', label: '>=' },
                    // { value: '<', label: '<' },
                    // { value: '<=', label: '<=' },
                    { value: 'CONTAINS', label: 'CONTAINS' },
                    { value: 'NOT CONTAINS', label: 'NOT CONTAINS' },
                    { value: 'STARTS WITH', label: 'STARTS WITH' },
                    { value: 'NOT STARTS WITH', label: 'NOT STARTS WITH' },
                    { value: 'IS EMPTY', label: 'IS EMPTY' },
                    { value: 'IS NOT EMPTY', label: 'IS NOT EMPTY' },
                    { value: 'EXISTS', label: 'EXISTS' },
                    { value: 'NOT EXISTS', label: 'NOT EXISTS' },

                    ]}
                    defaultValue="="
                    value={cond.operator}
                    onChange={val => {
                        const newConditions = [...conditions];
                        newConditions[idx] = { ...newConditions[idx], operator: val };
                        setConditions(newConditions);

                    }}
                />
                <Input
                    placeholder="Value"
                    value={cond.value}
                    disabled={cond.operator.includes("EXIST")}

                    onChange={e => {
                        const newConditions = [...conditions];
                        newConditions[idx] = { ...newConditions[idx], value: e.target.value };
                        setConditions(newConditions);

                    }}
                />

                <Select
                    options={[{ value: 'AND', label: 'AND' }, { value: 'OR', label: 'OR' }]}
                    defaultValue="AND"
                    value={cond.joiner}
                    onChange={e => {
                        const newConditions = [...conditions];
                        newConditions[idx] = { ...newConditions[idx], joiner: e };
                        setConditions(newConditions);

                    }}
                />
                <Button onClick={() => {
                    const newConds = conditions.filter((_, i) => i !== idx);
                    setConditions(newConds);

                }}>Remove</Button>
            </Flex>
        ))}


    </>
    );
};
export default FilterBuilder;
