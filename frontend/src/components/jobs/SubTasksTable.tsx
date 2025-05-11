import { useEffect, } from "react";
import { useNavigate, useParams } from "react-router-dom"; // For extracting path variables
import { Table, Tag, Button, TableProps, Flex, Space, Pagination, Typography } from "antd";
import { colorForStatus, Status, SubTask, truncate, } from "@swarm/models/domain";
import { ArrowLeftOutlined, DownloadOutlined, PlusOutlined } from "@ant-design/icons";
import { useDispatch, useSelector } from "react-redux";
import { AppDispatch, RootState } from "@swarm/states/Store";
import { fetchSubTasks, reset } from "@swarm/states/SuBTaskSlice";
import { download } from "@swarm/states/file/Api";
import dayjs from "dayjs";
import Link from "antd/es/typography/Link";
import { useAuth } from "@swarm/auth/authContextHook";

const { Text } = Typography;
const SubTasksTable: React.FC = () => {
    const { token } = useAuth();
    const navigate = useNavigate();
    const { id: jobId, taskId, taskName } = useParams<{ id: string; taskId: string, taskName: string }>();
    const dispatch = useDispatch<AppDispatch>();

    const { data, loading, lastElementId } = useSelector((state: RootState) => state.appReducer.subTasks);

    const pageSize = 50;
    useEffect(() => {
        dispatch(reset());
        if (jobId && taskId) {
            dispatch(fetchSubTasks({ jobId, taskId, lastElementId: null, pageSize }));
        }
    }, [dispatch, jobId, taskId, taskName]);
    const loadMore = () => {
        if (jobId && taskId) {
            dispatch(fetchSubTasks({ jobId, taskId, lastElementId, pageSize }));
        }
    };

    const columns: TableProps<SubTask>["columns"] = [
        {
            title: "Base url",
            key: "baseUrl",
            render: (_, record) => {
                if (record.result &&
                    (record.result.type === "nTriple" ||
                        record.result.type === "diff" ||
                        record.result?.type == "scrapeUrl")) {
                    return <a href={record.result.value.baseUrl} target="_blank">{record.result.value.baseUrl} </a>
                } else {
                    return (<>
                        N/A
                    </>)

                }
            }
        },
        {
            title: "Creation Date",
            dataIndex: "creationDate",
            key: "creationDate",
            render: (date: string) => dayjs(new Date(date)).format('DD/MM/YYYY HH:mm:ss'),
        },

        {
            title: "Status",
            dataIndex: "status",
            key: "status",
            render: (status: Status) => {
                return (
                    <Tag color={colorForStatus(status)} >
                        <Text>{status.type}
                            {status.type === "failed" && status.value && `: ${truncate(status.value.join(", "), 32)}`}</Text>
                    </Tag>
                );
            },
        },
        {
            title: "Download",
            key: "actions",
            align: "center",
            render: (_, record: SubTask) => {
                const res = record.result;
                let component = <>N/A</>;
                if (res) {
                    switch (res.type) {
                        case "scrapeUrl":
                            component = <Tag>
                                <Link disabled={!token} onClick={async () => await download(jobId, res.value.path)}><DownloadOutlined />file.html</Link>
                            </Tag>;
                            break;
                        case "diff":
                            component = <Space>
                                {res.value.toRemovePath && <Link disabled={!token} onClick={async () => await download(jobId, res.value.toRemovePath)}><DownloadOutlined />to-remove.ttl</Link>}
                                {res.value.intersectPath && <Link disabled={!token} onClick={async () => await download(jobId, res.value.intersectPath)}><DownloadOutlined />intersect.ttl</Link>}
                                {res.value.newInsertPath && <Link disabled={!token} onClick={async () => await download(jobId, res.value.newInsertPath)}><DownloadOutlined />new-inserts.ttl</Link>}
                            </Space>;
                            break;
                        case "nTriple":
                            component = <Tag>
                                <Link disabled={!token} onClick={async () => await download(jobId, res.value.path)}><DownloadOutlined />result.ttl</Link>
                            </Tag>;
                            break;
                    }
                }

                return component;

            },
        },

    ];

    return (
        <>
            <Flex vertical gap="middle">
                <Flex justify="space-between" wrap>
                    <h2>{taskName}</h2>
                    <Space>
                        <Button onClick={() => navigate(`/jobs/${jobId}/tasks`)} icon={<ArrowLeftOutlined />} size="large" color="default" variant="dashed">Back</Button>
                        <Button disabled={loading || !lastElementId} onClick={loadMore} size="large" color="default" variant="dashed" icon={<PlusOutlined />}>
                            Load {pageSize} more
                        </Button>
                    </Space>

                </Flex>
                <Table
                    bordered
                    dataSource={data}
                    columns={columns}
                    rowKey="_id"
                    loading={loading}

                >
                    <Pagination responsive pageSize={pageSize} />
                </Table>
            </Flex>

        </>

    );
};

export default SubTasksTable;
