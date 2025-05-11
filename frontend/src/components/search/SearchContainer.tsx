import { AppDispatch, RootState } from "@swarm/states/Store";
import { Alert, Button, Col, Descriptions, Divider, Flex, Input, Pagination, Row, Select, Space, Spin, Typography, } from "antd";
import { useEffect, useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import FilterBuilder from "./FilterBuilder";
import { clearSearchResult, fetchIndexStatistics, fetchSearchConfigurations, performSearch } from "@swarm/states/SearchSlice";
import { SearchQueryRequest } from "@swarm/models/domain";
const SearchContainer: React.FC = () => {

    const dispatch = useDispatch<AppDispatch>();

    const indexConfigs = useSelector((state: RootState) => state.appReducer.search.configurations);
    const indexStats = useSelector((state: RootState) => state.appReducer.search.indexStatistics);
    const searchResults = useSelector((state: RootState) => state.appReducer.search.searchResult);
    const loading = useSelector((state: RootState) => state.appReducer.search.loading);
    const searching = useSelector((state: RootState) => state.appReducer.search.searching);
    const error = useSelector((state: RootState) => state.appReducer.search.error);
    const [selectedIndex, setSelectedIndex] = useState<undefined | string>();
    const [query, setQuery] = useState<undefined | string>("");
    const [limit, _] = useState(5);
    const [filters, setFilters] = useState<{ key: string, operator: string, joiner: string, value: string }[]>([]);


    useEffect(() => { dispatch(fetchSearchConfigurations()) }, [dispatch]);
    useEffect(() => {
        dispatch(clearSearchResult());
    }, [selectedIndex, dispatch]);

    function updateIndex(index: undefined | string) {
        setSelectedIndex(index);
        dispatch(fetchIndexStatistics(index));
    }

    function handleSearch(page = 1) {
        if (!selectedIndex) return;
        let filter: undefined | string = undefined;
        const filteredFilters = filters.filter(f => f.key?.length);
        if (filteredFilters.length) {
            filter = filteredFilters.map((f, i) => {
                const value = f.value.split(' ').length > 1 || f.value.includes(".") ? `"${f.value}"` : f.value;
                const part = `${f.key} ${f.operator} ${value}`;
                return i < filteredFilters.length - 1 ? `${part} ${f.joiner}` : part;
            }).join(' ');
        }
        const req: SearchQueryRequest = {
            filters: filter ? `(${filter})` : undefined,
            query: query?.length ? query.split(' ').length > 1 ? { type: "phrase", value: query } : { type: "word", value: query } : undefined,
            limit,
            page,
            neg: false, // fixme sort not done, neg probably not needed
        };
        dispatch(performSearch({ index: selectedIndex!, request: req }));
    }

    return (<>
        <h2>INDEX</h2>
        <Flex justify="center" >
            <Col span={12}>

                <Flex gap="middle" >
                    <Select
                        size="large"


                        placeholder="Select type"
                        onChange={value => { updateIndex(value); setFilters([]); setQuery(undefined) }}
                        options={indexConfigs.map(ic => ({ value: ic.name, label: ic.name }))}
                    />
                    <Space />
                    <Input.Search
                        size="large"
                        disabled={!selectedIndex}
                        enterButton
                        onSearch={() => handleSearch()}
                        value={query}
                        placeholder="Enter query"
                        onChange={e => { setQuery(e.target.value) }}
                    />
                    <Button size="large" disabled={!selectedIndex} type="dashed" danger onClick={() => setFilters([...filters, { key: '', operator: '=', value: '', joiner: 'AND' }])}>
                        Add Filter
                    </Button>
                </Flex >
                {indexStats && (<Typography.Text type="secondary">(~{new Intl.NumberFormat("nl-BE").format(
                    indexStats.numberOfDocuments,
                )} document{indexStats.numberOfDocuments > 1 ? 's' : ''})</Typography.Text>)}
                {selectedIndex && (<>
                    <Row style={{ paddingTop: "10px" }}>
                        <FilterBuilder key={selectedIndex}
                            indexConfig={indexConfigs.find(ic => ic.name === selectedIndex)!}
                            conditions={filters}
                            setConditions={setFilters}
                        />
                    </Row>
                </>)
                }
                <Divider dashed />
            </Col>
        </Flex>

        {error &&
            <Alert
                message="Error"
                description={error}
                type="error"
                showIcon
                closable
            />
        }
        <Spin spinning={loading || searching}>
            {searchResults && (<>
                {/* <Typography.Title level={3} type="secondary">(Found {searchResults.totalHits || 0} document{(searchResults.totalHits || 0) > 1 ? 's' : ''})</Typography.Title> */}
                <Flex vertical gap="middle" style={{ paddingTop: "10px" }} >
                    {
                        searchResults.hits.map((hit, idx) => (
                            <Descriptions key={hit._id as string}
                                bordered
                                column={1}
                            >
                                {Object.entries(hit).map(([key, val]) => (
                                    <Descriptions.Item key={key + idx + hit._id} label={key} styles={{ label: { width: '10vw', fontWeight: "bold" } }}>
                                        {Array.isArray(val) ? (<ul key={key + idx + hit._id + "ul"} style={{ padding: 0, marginLeft: 10 }}>{val.map(v => <li key={crypto.randomUUID()}>{v}</li>)}</ul>) : <span style={{ wordBreak: 'break-word' }}>{String(val)}</span>
                                        }
                                    </Descriptions.Item>
                                ))}
                            </Descriptions>

                        ))
                    }
                    {(searchResults.totalPages || 1) > 1 && (
                        <Pagination responsive showSizeChanger={false} align="end"
                            current={searchResults.page || 1}
                            pageSize={limit}
                            total={searchResults.totalHits || (searchResults.totalPages || 1 * limit)}
                            onChange={p => { handleSearch(p) }}
                        />
                    )}
                </Flex > </>)
            }
        </Spin >



    </>

    );
};
export default SearchContainer;
