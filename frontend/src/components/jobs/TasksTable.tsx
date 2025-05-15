
import React, { useEffect, useState } from 'react';
import { Button, Flex, Space, Table, TableProps, Tag, Typography, message } from 'antd';
import { useNavigate, useParams } from 'react-router-dom';  // for accessing the dynamic route parameter
import axios from 'axios';
import { colorForStatus, Job, Status, Task, } from '@swarm/models/domain';
import { ArrowLeftOutlined, DownloadOutlined, RightSquareOutlined, SyncOutlined } from "@ant-design/icons";
import { download } from '@swarm/states/file/Api';
import dayjs from 'dayjs';
import { useAuth } from '@swarm/auth/authContextHook';
import Link from 'antd/es/typography/Link';
import { useIsMobile } from '@swarm/hooks/is-mobile';
import JobDetail from './JobDetail';


const TasksTable: React.FC = () => {
    const isMobile = useIsMobile();
    const navigate = useNavigate();
    const { token } = useAuth();
    const { id } = useParams<{ id: string }>();
    const [job, setJob] = useState<Job>();
    const [tasks, setTasks] = useState<Task[]>([]);
    const [loading, setLoading] = useState<boolean>(true);
    const fetchTasks = async (id: string | undefined) => {
        setLoading(true);
        try {
            const responseTasks = await axios.get(`/api/jobs/${id}/tasks`);
            const tasks = responseTasks.data;
            setTasks(tasks);
            const responseJob = await axios.get(`/api/jobs/${id}`);
            const job = responseJob.data;
            setJob(job);
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
            title: 'Created',
            dataIndex: 'creationDate',
            key: 'creationDate',
            render: (date: string) => dayjs(new Date(date)).format('DD/MM/YYYY HH:mm:ss'),
        },
        {
            title: 'Modified',
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
                                <Link disabled={!token} onClick={async () => await download(record.jobId, res.value.manifestFilePath)}><DownloadOutlined />manifest.json</Link></Tag>
                        </>
                    }
                    if (res.type == "publish") {
                        return <Flex vertical gap="small">
                            <Tag>
                                <Link disabled={!token} onClick={async () => await download(record.jobId, res.value.insertedTripleFilePath)}><DownloadOutlined />Inserted File</Link></Tag>
                            <Tag>
                                <Link disabled={!token} onClick={async () => await download(record.jobId, res.value.removedTripleFilePath)}><DownloadOutlined />Removed File</Link></Tag>
                            <Tag>
                                <Link disabled={!token} onClick={async () => await download(record.jobId, res.value.intersectTripleFilePath)}><DownloadOutlined />Intersect File</Link></Tag>
                            <Tag>
                                <Link disabled={!token} onClick={async () => await download(record.jobId, res.value.failedQueryFilePath)}><DownloadOutlined />Failed File</Link></Tag>
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
                <h2>Job Detail</h2>

                <Space>
                    <Button onClick={() => navigate("/jobs")} icon={<ArrowLeftOutlined />} size="large" color="default" variant="dashed">{!isMobile && 'Back'}</Button>
                    <Button onClick={() => fetchTasks(id)} size="large" color="default" variant="dashed" icon={<SyncOutlined />}>
                        {!isMobile && 'Refresh'}
                    </Button>
                </Space>
            </Flex>


            {job && <JobDetail job={job} />}

            <h2>Tasks</h2>

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
