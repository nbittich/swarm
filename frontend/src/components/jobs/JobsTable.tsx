
import React, { useEffect, useState } from 'react';
import { Table, Button, Space, Modal, Form, Input, message, Flex, Select, TableProps, Tag, Popconfirm, } from 'antd';
import { DeleteOutlined, PlusOutlined, RightSquareOutlined, SyncOutlined } from '@ant-design/icons';
import axios from 'axios';
import { colorForStatus, Job, JobDefinition, Status, statusOptions, TaskDefinition } from '@swarm/models/domain';
import { useNavigate } from 'react-router-dom';
import dayjs from 'dayjs';
const { Option } = Select;
const JobsTable: React.FC = () => {
  const navigate = useNavigate();
  const [jobs, setJobs] = useState<Job[]>([]);
  const [jobDefinitions, setJobDefinitions] = useState<JobDefinition[]>([]);
  const [taskDefinition, setTaskDefinition] = useState<TaskDefinition | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [isModalVisible, setIsModalVisible] = useState<boolean>(false);
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
    status?: string,
  }) => {
    try {
      setLoading(true);
      const payload = {
        definitionId: values.definitionId,
        jobName: values.jobName?.length ? values.jobName : undefined,
        taskDefinition,
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
      const response = await axios.post('/api/jobs/new', payload);
      message.success('Job added successfully');
      toggleModal(false);
      setJobs([response.data, ...jobs]);
    } catch (error) {
      console.error(error);
      message.error('Failed to add job. Check the console');
    } finally {
      setLoading(false);
    }
  };

  const deleteJob = async (job: Job) => {
    try {
      setLoading(true);
      await axios.delete('/api/jobs/' + job._id);
      message.success('Job deleted successfully');
      setJobs(jobs.filter((j) => job._id !== j._id));
    } catch (error) {
      console.error(error);
      message.error('Failed to delete job. Check the console');
    } finally {
      setLoading(false);
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
      title: 'Payload',
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
      render: (date: string) => dayjs(new Date(date)).format('DD/MM/YYYY HH:mm:ss'),
    },
    {
      title: 'Modified Date',
      dataIndex: 'modifiedDate',
      key: 'modifiedDate',
      render: (date?: string) => date ? dayjs(new Date(date)).format('DD/MM/YYYY HH:mm:ss') : 'N/A',
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
    {
      title: 'Action',
      key: 'action',
      align: 'center',
      render: (_, record) => (
        <Popconfirm
          placement='left'
          title="Delete the job"
          description="Are you sure to delete this job?"
          onConfirm={() => deleteJob(record)}
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
            <Button onClick={() => fetchJobsAndDefinitions()} size="large" color="default" variant="dashed" icon={<SyncOutlined />}>
              Refresh
            </Button>
          </Space>

        </Flex>
        <Table
          loading={loading}
          bordered
          dataSource={jobs}
          columns={columns}
          rowKey="_id"
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
              <Button onClick={() => form.submit()} type="dashed">Submit</Button></Space>
          </Flex >

        )}
      >
        <Form form={form} onFinish={handleAddJob} layout="vertical">

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
        </Form>
      </Modal>

    </>
  );
};

export default JobsTable;
