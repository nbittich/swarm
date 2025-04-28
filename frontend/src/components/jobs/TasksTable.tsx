
import React, { useEffect, useState } from 'react';
import { Button, Flex, Space, Table, TableProps, Tag, message } from 'antd';
import { useNavigate, useParams } from 'react-router-dom';  // for accessing the dynamic route parameter
import axios from 'axios';
import { colorForStatus, Status, Task } from '@swarm/models/domain';
import { ArrowLeftOutlined, DownloadOutlined, RightSquareOutlined, SyncOutlined } from "@ant-design/icons";
import { download } from '@swarm/states/file/Api';
import dayjs from 'dayjs';


const TasksTable: React.FC = () => {
    const navigate = useNavigate();
    const { id } = useParams<{ id: string }>();
    const [tasks, setTasks] = useState<Task[]>([]);
    const [loading, setLoading] = useState<boolean>(true);
    const fetchTasks = async (id: string | undefined) => {
        setLoading(true);
        try {
            const response = await axios.get(`/api/jobs/${id}/tasks`);
            const tasks = response.data;
            setTasks(tasks);
        } catch (err) {
            console.error(err);
            message.error('Failed to fetch tasks. Check the logs');
        } finally {
            setLoading(false);
        }
    };
    useEffect(() => {
        fetchTasks(id);
    }, [id]);

    const columns: TableProps<Task>["columns"] = [
        {
            title: 'Task Name',
            dataIndex: 'name',
            key: 'name',
            render: (name: string) => {
                return <>
                    <Tag >
                        {name}
                    </Tag>
                </>
            },
        },

        {
            title: 'Creation Date',
            dataIndex: 'creationDate',
            key: 'creationDate',
            render: (date: string) => dayjs(new Date(date)).format('DD/MM/YYYY HH:mm:ss'),
        },
        {
            title: 'Modified Date',
            dataIndex: 'modifiedDate',
            key: 'modifiedDate',
            render: (date?: string) => date ? dayjs(new Date(date)).format('DD/MM/YYYY HH:mm:ss') : 'N/A',
        },
        {
            title: 'Success',
            key: 'success',
            render: (_, record) => {
                if (record.result) {
                    const res = record.result;
                    if (res.type == "diff" || res.type == "extractRDFa" || res.type == "filterSHACL" || res.type == "scrapeWebsite" || res.type == "complementWithUuid") {
                        return <>
                            {res.value.successCount}
                        </>;
                    }
                }
                return 'N/A';
            },
        },
        {
            title: 'Failure',
            key: 'failure',
            render: (_, record) => {
                if (record.result) {
                    const res = record.result;
                    if (res.type == "diff" || res.type == "extractRDFa" || res.type == "filterSHACL" || res.type == "scrapeWebsite" || res.type == "complementWithUuid") {
                        return <>
                            {res.value.failureCount}
                        </>;
                    }
                }
                return 'N/A';
            },
        },
        {
            title: 'Result',
            key: 'result',
            render: (_, record) => {
                if (record.result) {
                    const res = record.result;
                    if (res.type == "diff" || res.type == "extractRDFa" || res.type == "filterSHACL" || res.type == "scrapeWebsite" || res.type == "complementWithUuid") {
                        return <>
                            <Tag>
                                <a onClick={async () => await download(record.jobId, res.value.manifestFilePath)}><DownloadOutlined />manifest.json</a></Tag>
                        </>
                    }
                    if (res.type == "publish") {
                        return <Flex vertical gap="small">
                            <Tag>
                                <a onClick={async () => await download(record.jobId, res.value.insertedTripleFilePath)}><DownloadOutlined />Inserted File</a></Tag>
                            <Tag>
                                <a onClick={async () => await download(record.jobId, res.value.removedTripleFilePath)}><DownloadOutlined />Removed File</a></Tag>
                            <Tag>
                                <a onClick={async () => await download(record.jobId, res.value.intersectTripleFilePath)}><DownloadOutlined />Intersect File</a></Tag>
                            <Tag>
                                <a onClick={async () => await download(record.jobId, res.value.failedQueryFilePath)}><DownloadOutlined />Failed File</a></Tag>
                        </Flex>

                    }
                }

                return "N/A";


            },
        },
        {
            title: 'Status',
            dataIndex: 'status',
            key: 'status',
            render: (status: Status) => {
                return <>
                    <Tag title={status.type == 'failed' ? status.value.join(", ") : undefined} color={colorForStatus(status)}>
                        {status.type}
                    </Tag>
                </>
            },
        },
        {
            title: 'SubTasks',
            key: 'subTasks',
            align: 'center',
            render: (_, record) => {
                if (record.hasSubTask) {
                    return (
                        <Button type="link" onClick={() => navigate(`/jobs/${record.jobId}/tasks/${record._id}/${record.name}`)} icon={<RightSquareOutlined />} />
                    )
                } else {
                    return 'N/A'
                }
            },
        },
    ];




    return (
        <Flex vertical gap="middle">
            <Flex justify="space-between" wrap>
                <h2>Tasks</h2>
                <Space>
                    <Button onClick={() => navigate("/jobs")} icon={<ArrowLeftOutlined />} size="large" color="default" variant="dashed">Back</Button>
                    <Button onClick={() => fetchTasks(id)} size="large" color="default" variant="dashed" icon={<SyncOutlined />}>
                        Refresh
                    </Button>
                </Space>
            </Flex>
            <Table
                bordered
                dataSource={tasks}
                columns={columns}
                rowKey="_id"
                loading={loading}
                scroll={{ x: 'max-content' }}
            />
        </Flex>
    );
};

export default TasksTable;
