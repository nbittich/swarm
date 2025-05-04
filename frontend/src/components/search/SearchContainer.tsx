import { AppDispatch, RootState } from "@swarm/states/Store";
import { Button, Card, Flex, Input, Pagination, Row, Select, Space } from "antd";
import { useEffect, useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import FilterBuilder from "./FilterBuilder";
import { fetchSearchConfigurations, performSearch } from "@swarm/states/SearchSlice";
import { SearchQueryRequest } from "@swarm/models/domain";

const SearchContainer: React.FC = () => {

    const dispatch = useDispatch<AppDispatch>();

    const indexConfigs = useSelector((state: RootState) => state.appReducer.search.configurations);
    const searchResults = useSelector((state: RootState) => state.appReducer.search.searchResult);
    const [selectedIndex, setSelectedIndex] = useState<string>();
    const [query, setQuery] = useState("");
    const [page, setPage] = useState(1);
    const [limit, _] = useState(5);
    const [filters, setFilters] = useState<{ key: string, operator: string, joiner: string, value: string }[]>([]);


    useEffect(() => { dispatch(fetchSearchConfigurations()) }, [dispatch]);

    function handleSearch() {
        let filter: undefined | string = undefined;
        if (filters.length) {
            filter = filters.map((f, i) => {
                const value = f.value.split(' ').length > 1 ? `"${f.value}"` : f.value;
                const part = `${f.key}${f.operator}${value}`;
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
        <Flex gap="middle">
            <Select
                placeholder="Select index"
                onChange={value => { setSelectedIndex(value); setFilters([]); }}
                options={indexConfigs.map(ic => ({ value: ic.name, label: ic.name }))}
            />
            <Space />
            <Input size="middle"
                placeholder="Enter query"
                onChange={e => { setQuery(e.target.value) }}
            />

            <Button type="dashed"
                onClick={() => handleSearch()}
                disabled={!selectedIndex}
            >Search</Button>
            {/* <SearchResult hits={hits} page={page} totalPages={totalPages} limit={10} onPageChange={handlePageChange} /> */}
            {/* {totalPages > 1 && ( */}
            {/*     <Pagination current={page} pageSize={10} total={totalPages * 10} onChange={handlePageChange} /> */}
            {/* )} */}
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
        {searchResults && (<div>
            {searchResults.hits.map((hit, idx) => (
                <Card title={`Record ${idx + 1}`} key={idx} style={{ margin: '10px 0' }}>
                    {Object.entries(hit).map(([key, val]) => (
                        <p key={key}>
                            <strong>{key}:</strong> <span style={{ wordBreak: 'break-word' }}>{String(val)}</span>
                        </p>
                    ))}
                </Card>
            ))}
            {(searchResults.totalPages || 1) > 1 && (
                <Pagination
                    current={page}
                    pageSize={limit}
                    total={searchResults.totalHits || (searchResults.totalPages || 1 * limit)}
                    onChange={page => { setPage(page); }}
                />
            )}
        </div>)}


    </>

    );
};
export default SearchContainer;
