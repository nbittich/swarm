import { AppDispatch, RootState } from "@swarm/states/Store";
import { Descriptions, Divider, Flex, Input, Pagination, Row, Select, Space, Spin, } from "antd";
import { useEffect, useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import FilterBuilder from "./FilterBuilder";
import { fetchSearchConfigurations, performSearch } from "@swarm/states/SearchSlice";
import { SearchQueryRequest } from "@swarm/models/domain";

const SearchContainer: React.FC = () => {

    const dispatch = useDispatch<AppDispatch>();

    const indexConfigs = useSelector((state: RootState) => state.appReducer.search.configurations);
    const searchResults = useSelector((state: RootState) => state.appReducer.search.searchResult);
    const loading = useSelector((state: RootState) => state.appReducer.search.loading);
    const searching = useSelector((state: RootState) => state.appReducer.search.searching);
    const [selectedIndex, setSelectedIndex] = useState<string>();
    const [query, setQuery] = useState<undefined | string>("");
    const [limit, _] = useState(5);
    const [filters, setFilters] = useState<{ key: string, operator: string, joiner: string, value: string }[]>([]);


    useEffect(() => { dispatch(fetchSearchConfigurations()) }, [dispatch]);

    function handleSearch(page = 1) {
        if (!selectedIndex) return;
        let filter: undefined | string = undefined;
        if (filters.length) {
            filter = filters.map((f, i) => {
                const value = f.value.split(' ').length > 1 || f.value.includes(".") ? `"${f.value}"` : f.value;
                const part = `${f.key} ${f.operator} ${value}`;
                return i < filters.length - 1 ? `${part} ${f.joiner}` : part;
            }).join(' ');
        }
        const req: SearchQueryRequest = {
            filters: filter,
            query: query?.length ? query.split(' ').length > 1 ? { type: "phrase", value: query } : { type: "word", value: query } : undefined,
            limit,
            page,
            neg: false, // fixme sort not done, neg probably not needed
        };
        dispatch(performSearch({ index: selectedIndex!, request: req }));
    }

    return (<>
        <h2>INDEX</h2>
        <Flex gap="middle" >
            <Select
                placeholder="Select index"
                onChange={value => { setSelectedIndex(value); setFilters([]); setQuery(undefined); handleSearch() }}
                options={indexConfigs.map(ic => ({ value: ic.name, label: ic.name }))}
            />
            <Space />
            <Input.Search size="middle"
                enterButton
                onSearch={() => handleSearch()}
                value={query}
                placeholder="Enter query"
                onChange={e => { setQuery(e.target.value) }}
            />

        </Flex>
        {selectedIndex && (<>
            <Row style={{ paddingTop: "10px" }}>
                <FilterBuilder
                    indexConfig={indexConfigs.find(ic => ic.name === selectedIndex)!}
                    onChange={setFilters}
                />
            </Row>
        </>)
        }
        <Spin spinning={loading || searching}>
            {searchResults && (<>
                <Divider />
                <Flex vertical gap="middle" style={{ paddingTop: "10px" }} >
                    {
                        searchResults.hits.map((hit, idx) => (
                            <Descriptions key={idx}
                                bordered
                                column={1}
                            >
                                {Object.entries(hit).map(([key, val]) => (
                                    <Descriptions.Item key={key} label={key} styles={{ label: { width: '10vw', fontWeight: "bold" } }}>
                                        {Array.isArray(val) ? (<ul style={{ padding: 0, marginLeft: 10 }}>{val.map(v => <li>{v}</li>)}</ul>) : <span style={{ wordBreak: 'break-word' }}>{String(val)}</span>
                                        }
                                    </Descriptions.Item>
                                ))}
                            </Descriptions>

                        ))
                    }
                    {(searchResults.totalPages || 1) > 1 && (
                        <Pagination
                            current={searchResults.page || 1}
                            pageSize={limit}
                            total={searchResults.totalHits || (searchResults.totalPages || 1 * limit)}
                            onChange={p => { handleSearch(p) }}
                        />
                    )}
                </Flex > </>)
            }
        </Spin>


    </>

    );
};
export default SearchContainer;
