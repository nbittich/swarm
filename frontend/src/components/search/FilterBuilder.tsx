import { IndexConfiguration } from "@swarm/models/domain";
import { Button, Flex, Input, Select, } from "antd";
import { useState } from "react";

interface FilterBuilderProps {
    indexConfig: IndexConfiguration;
    onChange: (filters: { key: string, operator: string, joiner: string, value: string }[]) => void;
}

const FilterBuilder: React.FC<FilterBuilderProps> = ({ indexConfig, onChange }) => {
    const [conditions, setConditions] = useState<{ key: string, operator: string, joiner: string, value: string }[]>([]
    );


    return (<>
        <Flex vertical gap="middle" style={{ width: "100%" }} >
            {conditions.map((cond, idx) => (
                <Flex key={idx} gap="middle" >
                    <Select
                        placeholder="Field"
                        style={{ width: "100%" }}
                        onChange={val => {
                            const newConditions = [...conditions];
                            newConditions[idx] = { ...newConditions[idx], key: val };
                            setConditions(newConditions);
                            onChange(newConditions);

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
                        { value: 'EXISTS', label: 'EXISTS' },
                        { value: 'NOT EXISTS', label: 'NOT EXISTS' },

                        ]}
                        defaultValue="="
                        value={cond.operator}
                        onChange={val => {
                            const newConditions = [...conditions];
                            newConditions[idx] = { ...newConditions[idx], operator: val };
                            setConditions(newConditions);
                            onChange(newConditions);

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
                            onChange(newConditions);

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

                            onChange(newConditions);

                        }}
                    />
                    <Button onClick={() => {
                        const newConds = conditions.filter((_, i) => i !== idx);
                        setConditions(newConds);
                        onChange(newConds);

                    }}>Remove</Button>
                </Flex>
            ))}
            <Button onClick={() => setConditions([...conditions, { key: '', operator: '=', value: '', joiner: 'AND' }])}>
                Add Filter
            </Button>
        </Flex>

    </>
    );
};
export default FilterBuilder;
