
import React, { useEffect, useState } from 'react';
import { Table, Button, Space, Modal, Form, Input, Flex, Select, TableProps, Tag, Popconfirm, PaginationProps, } from 'antd';
import { DeleteOutlined, PlusOutlined, RightSquareOutlined, SyncOutlined } from '@ant-design/icons';
import { colorForStatus, Job, statusOptions, TaskDefinition } from '@swarm/models/domain';
import { useNavigate } from 'react-router-dom';
import dayjs from 'dayjs';
import { fetchJobDefinitions } from '@swarm/states/JobDefinitionSlice';
import { useDispatch, useSelector } from 'react-redux';
import { AppDispatch, RootState } from '@swarm/states/Store';
import { SorterResult } from 'antd/es/table/interface';
import { addJob, deleteJob, fetchJobs, setPageable } from '@swarm/states/JobSlice';
import { useDebouncedCallback } from 'use-debounce';
const { Option } = Select;
const JobsTable: React.FC = () => {
    const navigate = useNavigate();
    const dispatch = useDispatch<AppDispatch>();
    const [searchName, setSearchName] = useState<undefined | string>();
    const searchNameDebounced = useDebouncedCallback(
        (e) => {
            setSearchName(e?.target?.value);
        },
        500
    );
    const [taskDefinition, setTaskDefinition] = useState<TaskDefinition | null>(null);
    const [isModalVisible, setIsModalVisible] = useState<boolean>(false);
    const { jobDefinitions, loading: jobDefLoading } = useSelector((state: RootState) => state.appReducer.jobDefinitions);
    const { jobs, pagination, loading: jobLoading, pageable } = useSelector((state: RootState) => state.appReducer.jobs);
    const [form] = Form.useForm();
    const toggleModal = (value: boolean) => {
        form.resetFields();
        setTaskDefinition(null);
        setIsModalVisible(value);
    }

    const handleJobDefinitionChange = (id: string) => {
        const selectedJD = jobDefinitions.find((jd) => jd.id === id);

        if (selectedJD && selectedJD.tasks.length > 0) {
            const firstTask = selectedJD.tasks[0];

            setTaskDefinition(firstTask);
        }
    };

    const handleTableChange = (newPagination: PaginationProps, _filters: Record<string, (React.Key | boolean)[] | null>,
        sorter: SorterResult<Job> | SorterResult<Job>[]) => {
        const sortParams: Record<string, 1 | -1> = {};

        if (Array.isArray(sorter)) {
            sorter.forEach(srt => {
                if (srt.order) {
                    sortParams[srt.field as string] = srt.order === 'ascend' ? 1 : -1;
                }
            });
        } else if (sorter.order) {
            sortParams[sorter.field as string] = sorter.order === 'ascend' ? 1 : -1;
        } else {
            sortParams["creationDate"] = -1;
        }

        dispatch(setPageable({ page: newPagination.current, limit: newPagination.pageSize, sort: sortParams }));
    };

    useEffect(() => {
        dispatch(fetchJobs(pageable));
    }, [pageable, dispatch]);
    useEffect(() => {
        dispatch(fetchJobDefinitions());
    }, [dispatch,]);
    useEffect(() => {
        if (searchName !== undefined)
            dispatch(setPageable({
                page: 1, filter: {
                    "name": {
                        "$regex": `${searchName}`,
                        "$options": "i"
                    }
                }
            }))
    }, [dispatch, searchName]);
    const handleAddJob = async (values: {
        definitionId: string,
        jobName?: string,
        targetUrl?: string,
        status?: string,
    }) => {

        const payload = {
            ...values,
            taskDefinition: taskDefinition
        };
        dispatch(addJob(payload));
        toggleModal(false);

    }
    const handleDeleteJob = async (job: Job) => {
        dispatch(deleteJob(job._id));
    };

    const columns: TableProps<Job>['columns'] = [
        {
            title: () => <Input placeholder='Name' onChange={searchNameDebounced}></Input>,
            dataIndex: 'name',
            width: '20%',
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
            title: 'Payload',
            width: '40%',
            dataIndex: 'payload',
            key: 'payload',
            render: (_, record) => {
                if (record.definition.tasks?.length) {
                    if (record.definition.tasks[0].payload.type === "scrapeUrl") {
                        const url = record.definition.tasks[0].payload.value;
                        return <a href={url} target="_blank">{url} </a>;
                    } else if (record.definition.tasks[0].payload.type === "cleanup") {
                        const status = record.definition.tasks[0].payload.value;
                        return <Tag>{status.type}</Tag>
                    }
                }
            }
        },
        {
            title: 'Creation Date',
            dataIndex: 'creationDate',
            key: 'creationDate',
            sorter: true,
            render: (date: string) => dayjs(new Date(date)).format('DD/MM/YYYY HH:mm:ss'),
        },
        {
            title: 'Modified Date',
            dataIndex: 'modifiedDate',
            key: 'modifiedDate',
            sorter: true,
            render: (date?: string) => date ? dayjs(new Date(date)).format('DD/MM/YYYY HH:mm:ss') : 'N/A',
        },
        {
            title: "Status",
            dataIndex: 'status',
            key: 'status.type',
            sorter: true,
            render: (_, record) => {
                return <>
                    <Tag title={record.status.type === "failed" ? record.status.value.join(", ") : undefined} color={colorForStatus(record.status)}>
                        {record.status.type}
                    </Tag>
                </>
            },
        },
        {
            title: 'Tasks',
            key: 'tasks',
            align: 'center',
            render: (_, record) => (
                <Button onClick={() => navigate(`/jobs/${record._id}/tasks`)} type="link" shape="default" icon={<RightSquareOutlined />} />
            ),
        },
        {
            title: 'Action',
            key: 'action',
            align: 'center',
            render: (_, record) => (
                <Popconfirm
                    placement='left'
                    title="Delete the job"
                    description="Are you sure to delete this job?"
                    onConfirm={() => handleDeleteJob(record)}
                    okText="Yes"
                    cancelText="No"
                >
                    <Button
                        disabled={["pending", "scheduled", "busy"].includes(record.status.type)}
                        type="link" shape="default" danger icon={<DeleteOutlined />} />
                </Popconfirm>
            ),
        },
    ];

    return (
        <>
            <Flex vertical gap="middle">
                <Flex justify="space-between" wrap>
                    <h2>Jobs</h2>
                    <Space>
                        <Button onClick={() => toggleModal(true)} size="large" color="default" variant="dashed" icon={<PlusOutlined />}>
                            New Job
                        </Button>
                        <Button onClick={() => handleTableChange({ current: 1 }, {}, {
                            field: "creationDate",
                            order: "descend",
                        })} size="large" color="default" variant="dashed" icon={<SyncOutlined />}>
                            Refresh
                        </Button>
                    </Space>

                </Flex>
                <Table
                    loading={jobLoading || jobDefLoading}
                    bordered
                    dataSource={jobs}
                    columns={columns}
                    pagination={pagination}
                    rowKey="_id"
                    onChange={handleTableChange}

                />
            </Flex >
            <Modal
                title="New Job"
                open={isModalVisible}
                onCancel={() => toggleModal(false)}
                footer={(
                    <Flex justify='end'>
                        <Space>
                            <Button onClick={(e) => {
                                e.stopPropagation()
                                toggleModal(false)
                            }}>Cancel</Button>
                            <Button onClick={() => form.submit()} type="dashed">Submit</Button></Space>
                    </Flex >

                )}
            >
                <Form form={form} onFinish={handleAddJob} layout="vertical">

                    <Form.Item
                        name="jobName"
                        label="Job Name">
                        <Input />
                    </Form.Item>
                    <Form.Item
                        name="definitionId"
                        label="Job Definition"
                        rules={[{ required: true, message: 'Please select a job definition' }]}
                    >
                        <Select placeholder="Select Job Definition" onChange={handleJobDefinitionChange}>
                            {jobDefinitions.map((definition) => (
                                <Option key={definition.id} value={definition.id}>
                                    {definition.name}
                                </Option>
                            ))}
                        </Select>
                    </Form.Item>


                    {taskDefinition && taskDefinition.payload.type === "scrapeUrl" && (
                        <Form.Item
                            name="targetUrl"
                            label="Target Url"
                            rules={[
                                { type: "url", message: "Please enter a valid url" },
                                { required: true, message: "url is required" },
                            ]}
                        >
                            <Input />
                        </Form.Item>
                    )}
                    {taskDefinition && taskDefinition.payload.type === "cleanup" && (
                        <Form.Item
                            name="status"
                            label="Status"
                            rules={[{ required: true, message: "Status is required" }]}
                        >
                            <Select placeholder="Select status">
                                {statusOptions.map((status) => (
                                    <Option value={status.type} key={status.type}>
                                        {status.type}
                                    </Option>
                                ))}
                            </Select>
                        </Form.Item>
                    )}
                </Form>
            </Modal>

        </>
    );
};

export default JobsTable;
