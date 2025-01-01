import React, { useEffect, useState } from 'react';
import { Table, Button, Space, Modal, Form, Input, message, Flex, Select, TableProps, Spin, Popconfirm, } from 'antd';
import { DeleteOutlined, PlusOutlined, SyncOutlined } from '@ant-design/icons';
import axios from 'axios';
import { JobDefinition, ScheduledJob, } from '@swarm/models/domain';
import cron from 'cron-validate';
const { Option } = Select;
const ScheduledJobsTable: React.FC = () => {
  const [scheduledJobs, setScheduledJobs] = useState<ScheduledJob[]>([]);
  const [jobDefinitions, setJobDefinitions] = useState<JobDefinition[]>([]);
  const [loading, setLoading] = useState<boolean>(false);
  const [isModalVisible, setIsModalVisible] = useState<boolean>(false);
  const [form] = Form.useForm();

  const toggleModal = (value: boolean) => {
    form.resetFields();
    setIsModalVisible(value);
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
  }) => {
    try {

      setLoading(true);

      const payload = {
        definitionId: values.definitionId,
        name: values.jobName,
        cronExpr: values.cronExpr,
        targetUrl: values.targetUrl,
      };

      const response = await axios.post('/api/scheduled-jobs/new', payload);
      message.success('Scheduled Job added successfully');
      toggleModal(false);
      form.resetFields();
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
      title: 'Target URL',
      dataIndex: 'targetUrl',
      key: 'targetUrl',
      render: (targetUrl?: string) => targetUrl ? <a href={targetUrl} target="_blank">{targetUrl} </a> : 'N/A'
    },
    {
      title: 'Creation Date',
      dataIndex: 'creationDate',
      key: 'creationDate',
      render: (date: string) => new Date(date).toLocaleString(),
    },
    {
      title: 'Next execution',
      dataIndex: 'nextExecution',
      key: 'nextExecution',
      render: (date?: string) => date ? new Date(date).toLocaleString() : 'N/A',
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
      <Spin spinning={loading}>
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
            bordered
            dataSource={scheduledJobs}
            columns={columns}
            rowKey="_id"
            pagination={{ pageSize: 10 }}
          />
        </Flex>
      </Spin>

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
            <Select placeholder="Select Job Definition">
              {jobDefinitions.map((definition) => (
                <Option key={definition.id} value={definition.id}>
                  {definition.name}
                </Option>
              ))}
            </Select>
          </Form.Item>

          <Form.Item
            name="targetUrl"
            label="Target URL"
            rules={[{ required: true, message: 'Please set a target url' },
            {
              type: "url",
              message: "This field must be a valid url."
            }]}

          >
            <Input type='url' />
          </Form.Item>

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
