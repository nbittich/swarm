import React, { useEffect, useState } from 'react';
import { Table, Button, Space, Modal, Form, Input, message, Flex, Select, TableProps, Popconfirm, Tag, } from 'antd';
import { DeleteOutlined, PlusOutlined, SyncOutlined } from '@ant-design/icons';
import axios from 'axios';
import { JobDefinition, ScheduledJob, statusOptions, TaskDefinition, } from '@swarm/models/domain';
import cron from 'cron-validate';
import dayjs from 'dayjs';
const { Option } = Select;
const ScheduledJobsTable: React.FC = () => {
  const [scheduledJobs, setScheduledJobs] = useState<ScheduledJob[]>([]);
  const [jobDefinitions, setJobDefinitions] = useState<JobDefinition[]>([]);
  const [taskDefinition, setTaskDefinition] = useState<TaskDefinition | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [isModalVisible, setIsModalVisible] = useState<boolean>(false);
  const [form] = Form.useForm();

  const toggleModal = (value: boolean) => {
    form.resetFields();
    setTaskDefinition(null);
    setIsModalVisible(value);
  };

  const handleJobDefinitionChange = (id: string) => {
    const selectedJD = jobDefinitions.find((jd) => jd.id === id);

    if (selectedJD && selectedJD.tasks.length > 0) {
      const firstTask = selectedJD.tasks[0];

      setTaskDefinition(firstTask);
    }
  };
  const deleteScheduledJob = async (job: ScheduledJob) => {
    try {
      setLoading(true);
      await axios.delete('/api/scheduled-jobs/' + job._id);
      message.success('Scheduled Job deleted successfully');
      setScheduledJobs(scheduledJobs.filter((j) => job._id !== j._id));
    } catch (error) {
      console.error(error);
      message.error('Failed to delete scheduled job. Check the console');
    } finally {
      setLoading(false);
    }
  };
  const fetchScheduledJobs = async () => {
    setLoading(true);
    try {
      const responseJobs = await axios.get('/api/scheduled-jobs');
      setScheduledJobs(responseJobs.data);

      const responseDefinitions = await axios.get('/api/job-definitions');
      setJobDefinitions(responseDefinitions.data);
    } catch (error) {
      console.error(error);
      message.error('Failed to fetch scheduled jobs');
    } finally {
      setLoading(false);
    }
  };
  useEffect(() => {
    fetchScheduledJobs();
  }, []);

  const handleAddScheduledJob = async (values: {
    jobName?: string,
    definitionId: string,
    cronExpr: string,
    targetUrl?: string,
    status?: string,
  }) => {
    try {

      setLoading(true);

      const payload = {
        definitionId: values.definitionId,
        name: values.jobName?.length ? values.jobName : undefined,
        cronExpr: values.cronExpr,
        taskDefinition
      };
      if (taskDefinition && taskDefinition.payload.type === "scrapeUrl" && values.targetUrl) {
        payload.taskDefinition = {
          ...taskDefinition,
          payload: {
            type: "scrapeUrl",
            value: values.targetUrl
          },

        };
      } else if (taskDefinition && taskDefinition.payload.type === "cleanup" && values.status && statusOptions.some(s => s.type === values.status)) {
        payload.taskDefinition = {
          ...taskDefinition,
          payload: {
            type: "cleanup",
            value: statusOptions.find(s => s.type === values.status)!
          },

        };
      } else {
        throw Error("invalid payload");
      }
      const response = await axios.post('/api/scheduled-jobs/new', payload);
      message.success('Scheduled Job added successfully');
      toggleModal(false);
      setScheduledJobs([response.data, ...scheduledJobs]);
    } catch (error) {
      console.error(error);
      message.error('Failed to add scheduled job. Check the console');
    } finally {
      setLoading(false);

    }
  };

  // Columns for the Ant Design table
  const columns: TableProps<ScheduledJob>['columns'] = [
    {
      title: 'Name',
      dataIndex: 'name',
      key: 'name',
      render: (name?: string) => name || 'N/A'
    },
    {
      title: 'Payload',
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
      title: 'Creation Date',
      dataIndex: 'creationDate',
      key: 'creationDate',
      render: (date: string) => dayjs(new Date(date)).format('DD/MM/YYYY HH:mm:ss'),
    },
    {
      title: 'Next execution',
      dataIndex: 'nextExecution',
      key: 'nextExecution',
      render: (date?: string) => date ? dayjs(new Date(date)).format('DD/MM/YYYY HH:mm:ss') : 'N/A',
    },
    {
      title: "Cron Expression",
      dataIndex: "cronExpr",
      key: "cronExpr"
    },
    {
      title: 'Action',
      key: 'action',
      align: 'center',
      render: (_, record) => (
        <Popconfirm
          placement='left'
          title="Delete the scheduled job"
          description="Are you sure to delete this scheduled job?"
          onConfirm={() => deleteScheduledJob(record)}
          okText="Yes"
          cancelText="No"
        >
          <Button type="link" shape="default" danger icon={<DeleteOutlined />} />
        </Popconfirm>
      ),
    }
  ];

  return (
    <>
      <Flex vertical gap="middle">
        <Flex justify="space-between" wrap>
          <h2>Scheduled Jobs</h2>
          <Space>
            <Button onClick={() => toggleModal(true)} size="large" color="default" variant="dashed" icon={<PlusOutlined />}>
              New Scheduled Job
            </Button>
            <Button onClick={() => fetchScheduledJobs()} size="large" color="default" variant="dashed" icon={<SyncOutlined />}>
              Refresh
            </Button>
          </Space>

        </Flex>
        <Table
          loading={loading}
          bordered
          dataSource={scheduledJobs}
          columns={columns}
          rowKey="_id"
          pagination={{ pageSize: 10 }}
        />
      </Flex>

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
            rules={[
              {
                pattern: /^[A-Za-z][A-Za-z0-9]*$/,
                message: 'Job name must be alphanumeric and start with a letter',
              },
            ]}
          >
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
