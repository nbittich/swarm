import { AppDispatch, RootState } from "@swarm/states/Store";
import { Alert, Button, Col, Divider, Empty, Flex, Input, List, Select, Space, Spin, Typography, } from "antd";
import { ReactElement, useEffect, useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import FilterBuilder from "./FilterBuilder";
import { clearSearchResult, fetchIndexStatistics, fetchSearchConfigurations, performSearch } from "@swarm/states/SearchSlice";
import { IndexConfiguration, SearchQueryRequest } from "@swarm/models/domain";
import { useIsMobile } from "@swarm/hooks/is-mobile";
import dayjs from "dayjs";

function customSort(arr: string[]) {
    return arr.sort((a, b) => {
        if (/^\d/.test(a) && /^\d/.test(b)) {
            const numA = parseFloat(a);
            const numB = parseFloat(b);
            if (numA !== numB) return numA - numB;
        }
        return a.localeCompare(b);
    });
}

function formatValue(indexConfig: IndexConfiguration | undefined, key: string, value: unknown): ReactElement {
    const config = indexConfig?.properties.find(p => p.name === key)?.config?.jsType;
    if (!config) {
        return <>{String(value)}</>;
    }
    switch (config) {
        case "number":
        case "string": return <>{String(value)}</>;
        case "url": return (<a target="_blank" href={String(value)}>{String(value)}</a>);
        case "date": return <>{dayjs(new Date(Number(value) * 1000)).format('DD/MM/YYYY HH:mm:ss')}</>
    }

}
const SearchContainer: React.FC = () => {

    const isMobile = useIsMobile();
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
    const [offset, setOffset] = useState(0);
    const [filters, setFilters] = useState<{ key: string, operator: string, joiner: string, value: string }[]>([]);

    useEffect(() => {
        dispatch(fetchSearchConfigurations());
    }, [dispatch]);

    useEffect(() => {
        dispatch(clearSearchResult());
    }, [selectedIndex, dispatch]);

    function updateIndex(index: undefined | string) {
        setSelectedIndex(index);
        dispatch(fetchIndexStatistics(index));
    }

    function handleSearch(off = 0) {
        if (!selectedIndex) return;
        setOffset(off);
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
            offset,
            neg: false, // fixme sort not done, neg probably not needed
        };
        dispatch(performSearch({ index: selectedIndex!, request: req }));
    }

    return (<>
        <Typography.Title style={{ textAlign: 'center' }}>
            <span style={{ color: '#8ecae6' }}>S</span>
            <span style={{ color: '#219ebc' }}>W</span>
            <span style={{ color: '#ffb703' }}>A</span>
            <span style={{ color: '#fb8500' }}>R</span>
            <span style={{ color: '#ff5d8f' }}>M</span>
        </Typography.Title>
        <Flex justify="center" >


            <Col span={isMobile ? 24 : 14}>
                <Flex vertical={isMobile} gap="middle" >
                    <Select
                        size="large"
                        placeholder="Select type"
                        value={selectedIndex}
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
                        placeholder="Search..."
                        onChange={e => { setQuery(e.target.value) }}
                    />
                    <Button size="large" disabled={!selectedIndex} type="dashed" danger onClick={() => setFilters([...filters, { key: '', operator: '=', value: '', joiner: 'AND' }])}>
                        Add Filter
                    </Button>

                    <Button size="large" disabled={!selectedIndex} type="dashed" onClick={() => updateIndex(undefined)}>
                        Clear
                    </Button>
                </Flex >
                {indexStats && (<Typography.Text type="secondary">(~{new Intl.NumberFormat("nl-BE").format(
                    indexStats.numberOfDocuments,
                )} document{indexStats.numberOfDocuments > 1 ? 's' : ''})</Typography.Text>)}
                {selectedIndex && (<>
                    <FilterBuilder key={selectedIndex}
                        indexConfig={indexConfigs.find(ic => ic.name === selectedIndex)!}
                        conditions={filters}
                        setConditions={setFilters}
                    />
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
        <Flex justify="center">
            <Col span={isMobile ? 24 : 14}>
                <Spin spinning={loading || searching}>
                    {searchResults && selectedIndex && (<>
                        {searchResults.hits.map((hit) => (
                            <List
                                key={hit._id as string}
                                bordered
                                style={{
                                    marginBottom: '10px',
                                    width: '100%',
                                }}
                                dataSource={Object.entries(hit).filter(([k, _]) => {
                                    const config = indexConfigs.find(ic => ic.name == selectedIndex)?.properties?.find(p => p.name === k)?.config;
                                    if (config) {
                                        return config.visible;
                                    }
                                    return true
                                }).sort()}
                                renderItem={([key, val]) => (
                                    <List.Item
                                        style={{
                                            display: 'flex',
                                            flexDirection: 'column',
                                            alignItems: "start",
                                            wordBreak: 'break-word',
                                        }}
                                    >
                                        <span
                                            style={{
                                                fontWeight: 'bold',
                                                paddingRight: '10px'
                                            }}
                                        >
                                            {key}
                                        </span>
                                        <span>
                                            {Array.isArray(val) ? (
                                                <ul>
                                                    {customSort([...val]).map((v) => (
                                                        <li key={crypto.randomUUID()}>{formatValue(indexConfigs.find(ic => ic.name == selectedIndex), key, v)}</li>
                                                    ))}
                                                </ul>
                                            ) : (
                                                formatValue(indexConfigs.find(ic => ic.name == selectedIndex), key, val)
                                            )}
                                        </span>
                                    </List.Item>
                                )}
                            />
                        ))}
                        {searchResults.hits && (
                            <Flex justify="end">
                                <Space>
                                    <Button disabled={offset == 0} onClick={(_) => handleSearch(offset - ((offset || 1) * limit))}>
                                        Prev
                                    </Button>
                                    <Button disabled={searchResults.hits.length < limit} onClick={(_) => handleSearch(offset + ((offset || 1) * limit))}>
                                        Next
                                    </Button>
                                </Space>
                            </Flex>
                        )
                        }</>)

                    }
                </Spin>
            </Col>

        </Flex>
        <Flex justify="center">
            <Col span={isMobile ? 24 : 14}>
                {!searchResults?.hits?.length && !loading && !searching && (
                    <Empty
                        image={Empty.PRESENTED_IMAGE_SIMPLE}
                        description={
                            <div>
                                <p>Use the dropdown above to select a dataset, then press search to begin.</p>
                            </div>
                        }
                    />
                )}
            </Col>
        </Flex>


    </>

    );
};
export default SearchContainer;
