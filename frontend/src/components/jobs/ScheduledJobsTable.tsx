import React, { useEffect, useState } from 'react';
import { Table, Button, Space, Modal, Form, Input, Flex, Select, TableProps, Popconfirm, Tag, PaginationProps, } from 'antd';
import { DeleteOutlined, PlusOutlined, SyncOutlined } from '@ant-design/icons';
import { ScheduledJob, statusOptions, TaskDefinition, } from '@swarm/models/domain';
import cron from 'cron-validate';
import dayjs from 'dayjs';
import { useDispatch, useSelector } from 'react-redux';
import { AppDispatch, RootState } from '@swarm/states/Store';
import { addScheduledJob, runScheduledJobManually, deleteScheduledJob, fetchScheduledJobs, setPageable } from '@swarm/states/ScheduledJobSlice';
import { fetchJobDefinitions } from '@swarm/states/JobDefinitionSlice';
import { useDebouncedCallback } from 'use-debounce';
import { SorterResult } from 'antd/es/table/interface';
import { useAuth } from '@swarm/auth/authContextHook';
import { useIsMobile } from '@swarm/hooks/is-mobile';
const { Option } = Select;
const ScheduledJobsTable: React.FC = () => {
    const isMobile = useIsMobile();
    const { token } = useAuth();
    const dispatch = useDispatch<AppDispatch>();
    const [searchName, setSearchName] = useState();
    const searchNameDebounced = useDebouncedCallback(
        (e) => {
            setSearchName(e?.target?.value);
        },
        500
    );
    const { jobDefinitions, loading: jobDefLoading } = useSelector((state: RootState) => state.appReducer.jobDefinitions);
    const { scheduledJobs, pagination, loading: scheduledJobLoading, pageable } = useSelector((state: RootState) => state.appReducer.scheduledJobs);
    const [taskDefinition, setTaskDefinition] = useState<TaskDefinition | null>(null);
    const [isModalVisible, setIsModalVisible] = useState<boolean>(false);
    const [form] = Form.useForm();

    const handleJobDefinitionChange = (id: string) => {
        const selectedJD = jobDefinitions.find((jd) => jd.id === id);

        if (selectedJD && selectedJD.tasks.length > 0) {
            const firstTask = selectedJD.tasks[0];

            setTaskDefinition(firstTask);
        }
    };

    const toggleModal = (value: boolean) => {
        form.resetFields();
        setTaskDefinition(null);
        setIsModalVisible(value);
    };

    const handleAddScheduledJob = async (values: {
        jobName?: string,
        definitionId: string,
        cronExpr: string,
        targetUrl?: string,
        status?: string,
    }) => {
        const payload = {
            ...values,
            taskDefinition: taskDefinition
        };
        dispatch(addScheduledJob(payload));
        toggleModal(false);
    };

    const deleteJob = (job: ScheduledJob) => {
        dispatch(deleteScheduledJob(job._id));
    };

    const runJob = (job: ScheduledJob) => {
        dispatch(runScheduledJobManually(job._id));
    };


    const handleTableChange = (newPagination: PaginationProps, _filters: Record<string, (React.Key | boolean)[] | null>,
        sorter: SorterResult<ScheduledJob> | SorterResult<ScheduledJob>[]) => {
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


    useEffect(() => {
        dispatch(fetchJobDefinitions());
    }, [dispatch,]);

    useEffect(() => {
        dispatch(fetchScheduledJobs(pageable));
    }, [pageable, dispatch]);

    const columns: TableProps<ScheduledJob>['columns'] = [
        {
            title: () => <Input placeholder='Name' onChange={searchNameDebounced}></Input>,
            dataIndex: 'name',
            responsive: ['sm'],
            width: '20%',
            key: 'name',
            render: (name?: string) => name || 'N/A'
        },
        {
            title: 'Payload',
            width: '40%',
            dataIndex: 'payload',
            key: 'payload',
            render: (_, record) => {
                if (record.taskDefinition.payload.type === "scrapeUrl") {
                    const url = record.taskDefinition.payload.value;
                    return <a href={url} target="_blank">{url} </a>;
                } else if (record.taskDefinition.payload.type === "cleanup") {
                    const status = record.taskDefinition.payload.value;
                    return <Tag>{status.type}</Tag>
                }
            }
        },
        {
            title: 'Created',
            responsive: ['sm'],
            dataIndex: 'creationDate',
            key: 'creationDate',
            sorter: true,
            render: (date: string) => dayjs(new Date(date)).format('DD/MM/YYYY HH:mm:ss'),
        },
        {
            title: 'Next',
            responsive: ['sm'],
            dataIndex: 'nextExecution',
            key: 'nextExecution',
            sorter: true,
            render: (date?: string) => date ? dayjs(new Date(date)).format('DD/MM/YYYY HH:mm:ss') : 'N/A',
        },
        {
            title: "Cron",
            dataIndex: "cronExpr",
            key: "cronExpr"
        },
        {
            title: 'Action',
            key: 'action',
            align: 'center',
            render: (_, record) => (<>                <Popconfirm
                placement='left'
                title="Delete the scheduled job"
                description="Are you sure to delete this scheduled job?"
                onConfirm={() => deleteJob(record)}
                okText="Yes"
                cancelText="No"
            >
                <Button disabled={!token} type="link" shape="default" danger icon={<DeleteOutlined />} />
            </Popconfirm>
                <Popconfirm
                    placement='right'
                    title="Run the scheduled job"
                    description="Are you sure to run this scheduled job?"
                    onConfirm={() => runJob(record)}
                    okText="Yes"
                    cancelText="No"
                >
                    <Button disabled={!token} type="link" shape="default" danger icon={<DeleteOutlined />} />
                </Popconfirm></>

            ),
        }
    ];

    return (
        <>
            <Flex justify="space-between" >
                <h2>Scheduled Jobs</h2>
                {token && <Space>
                    <Button onClick={() => toggleModal(true)} size="large" color="default" variant="dashed" icon={<PlusOutlined />}>
                        {isMobile ? '' : 'New Scheduled Job'}
                    </Button>
                    <Button onClick={() => handleTableChange({ current: 1 }, {}, {
                        field: "creationDate",
                        order: "descend",
                    })} size="large" color="default" variant="dashed" icon={<SyncOutlined />}>
                        {isMobile ? '' : 'Refresh'}
                    </Button>
                </Space>}

            </Flex>
            <Table
                pagination={pagination}
                scroll={{ x: 'max-content' }}
                loading={jobDefLoading || scheduledJobLoading}
                bordered
                dataSource={scheduledJobs}
                columns={columns}
                onChange={handleTableChange}
                rowKey="_id"
            />

            <Modal
                title="New Scheduled Job"
                open={isModalVisible}
                onCancel={() => toggleModal(false)}
                footer={(
                    <Flex justify='end'>
                        <Space>
                            <Button onClick={(e) => {
                                e.stopPropagation()
                                toggleModal(false)
                            }}>Cancel</Button>
                            <Button onClick={() => form.submit()} type="primary">Submit</Button></Space>
                    </Flex>

                )}
            >
                <Form form={form} onFinish={handleAddScheduledJob} layout="vertical">
                    <Form.Item
                        name="jobName"
                        label="Job Name"
                    >
                        <Input />
                    </Form.Item>
                    <Form.Item
                        name="definitionId"
                        label="Job Definition"
                        rules={[{ required: true, message: 'Please select a job definition' }]}
                    >
                        <Select placeholder="Select Job Definition" onChange={handleJobDefinitionChange}>
                            {jobDefinitions && jobDefinitions.map((definition) => (
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

                    <Form.Item
                        name="cronExpr"
                        label="Cron Expression"
                        rules={[{ required: true, message: 'Please set a cron expression' },
                        () => ({
                            validator(_, value) {
                                if (!value?.length) {
                                    return Promise.resolve();
                                }
                                const res = cron(value, { override: { useAliases: true, useYears: true, useSeconds: true } });
                                if (res.isValid()) {
                                    return Promise.resolve();
                                } else {
                                    return Promise.reject(new Error(res.getError().join(',')));
                                }
                            },
                        }),
                        ]}
                    >
                        <Input />
                    </Form.Item>
                </Form>
            </Modal>

        </>
    );
};

export default ScheduledJobsTable;
