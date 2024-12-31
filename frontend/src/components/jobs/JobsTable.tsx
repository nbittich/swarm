
import React, { useEffect, useState } from 'react';
import { Table, Button, Space, Modal, Form, Input, message, Flex, Select, TableProps, Tag, } from 'antd';
import { PlusOutlined, RightSquareOutlined, SyncOutlined } from '@ant-design/icons';
import axios from 'axios';
import { colorForStatus, Job, JobDefinition, Status } from '@swarm/models/domain';
import { useNavigate } from 'react-router-dom';
const { Option } = Select;
const JobsTable: React.FC = () => {
  const navigate = useNavigate();
  const [jobs, setJobs] = useState<Job[]>([]);
  const [jobDefinitions, setJobDefinitions] = useState<JobDefinition[]>([]);
  const [loading, setLoading] = useState<boolean>(false);
  const [isModalVisible, setIsModalVisible] = useState<boolean>(false);
  const [form] = Form.useForm();
  const toggleModal = (value: boolean) => {
    form.resetFields();
    setIsModalVisible(value);
  }
  const fetchJobsAndDefinitions = async () => {
    setLoading(true);
    try {
      const responseJobs = await axios.get('/api/jobs');
      setJobs(responseJobs.data);

      const responseDefinitions = await axios.get('/api/job-definitions');
      setJobDefinitions(responseDefinitions.data);

    } catch (error) {
      console.error(error);
      message.error('Failed to fetch jobs');
    } finally {
      setLoading(false);
    }
  };
  useEffect(() => {
    fetchJobsAndDefinitions();
  }, []);

  const handleAddJob = async (values: {
    definitionId: string,
    jobName?: string,
    targetUrl?: string,
  }) => {
    try {
      // Prepare the payload based on the form data
      const payload = {
        definitionId: values.definitionId,
        jobName: values.jobName,
        targetUrl: values.targetUrl,
      };

      const response = await axios.post('/api/jobs/new', payload);
      message.success('Job added successfully');
      toggleModal(false);
      form.resetFields();
      setJobs([response.data, ...jobs]);
    } catch (error) {
      console.error(error);
      message.error('Failed to add job. Check the console');
    }
  };

  // Columns for the Ant Design table
  const columns: TableProps<Job>['columns'] = [
    {
      title: 'Job Name',
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
      title: 'Target URL',
      dataIndex: 'targetUrl',
      key: 'targetUrl',
      render: (url?: string) => url ? <a href={url} target="_blank">{url} </a> : 'N/A'
    },
    {
      title: 'Creation Date',
      dataIndex: 'creationDate',
      key: 'creationDate',
      render: (date: string) => new Date(date).toLocaleString(),
    },
    {
      title: 'Modified Date',
      dataIndex: 'modifiedDate',
      key: 'modifiedDate',
      render: (date?: string) => date ? new Date(date).toLocaleString() : 'N/A',
    },
    {
      title: 'Status',
      dataIndex: 'status',
      key: 'status',
      render: (status: Status) => {
        return <>
          <Tag title={status.type === "failed" ? status.value.join(", ") : undefined} color={colorForStatus(status)}>
            {status.type}
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
            <Button onClick={() => fetchJobsAndDefinitions()} size="large" color="default" variant="dashed" icon={<SyncOutlined />}>
              Refresh
            </Button>
          </Space>

        </Flex>
        <Table
          bordered
          dataSource={jobs}
          columns={columns}
          rowKey="_id"
          loading={loading}
          pagination={{ pageSize: 10 }}
        />
      </Flex>
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
              <Button onClick={() => form.submit()} type="primary">Submit</Button></Space>
          </Flex >

        )}
      >
        <Form form={form} onFinish={handleAddJob} layout="vertical">
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
          {/**/}
          {/* <Form.Item */}
          {/*   name="jobName" */}
          {/*   label="Job Name" */}
          {/* > */}
          {/*   <Input /> */}
          {/* </Form.Item> */}

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

        </Form>
      </Modal>

    </>
  );
};

export default JobsTable;
