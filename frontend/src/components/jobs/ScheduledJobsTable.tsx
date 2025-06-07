import React, { useEffect, useState } from "react";
import {
  Table,
  Button,
  Space,
  Input,
  Flex,
  TableProps,
  Popconfirm,
  Tag,
  PaginationProps,
} from "antd";
import {
  DeleteOutlined,
  EditOutlined,
  PlusOutlined,
  PauseOutlined,
  PlaySquareOutlined,
  SyncOutlined,
  ApiOutlined,
} from "@ant-design/icons";
import {
  getPayloadFromScheduledJob,
  ScheduledJob,
  TaskDefinition,
} from "@swarm/models/domain";
import dayjs from "dayjs";
import { useDispatch, useSelector } from "react-redux";
import { AppDispatch, RootState } from "@swarm/states/Store";
import {
  upsertScheduledJob,
  runScheduledJobManually,
  deleteScheduledJob,
  fetchScheduledJobs,
  setPageable,
  fetchScheduledJobStatus,
  toggleScheduledJobStatus,
} from "@swarm/states/ScheduledJobSlice";
import { fetchJobDefinitions } from "@swarm/states/JobDefinitionSlice";
import { useDebouncedCallback } from "use-debounce";
import { SorterResult } from "antd/es/table/interface";
import { useAuth } from "@swarm/auth/authContextHook";
import { useIsMobile } from "@swarm/hooks/is-mobile";
import UpsertScheduledJob, { UpsertType } from "./UpsertScheduledJob";
const ScheduledJobsTable: React.FC = () => {
  const isMobile = useIsMobile();
  const { token } = useAuth();
  const dispatch = useDispatch<AppDispatch>();
  const [searchName, setSearchName] = useState();
  const searchNameDebounced = useDebouncedCallback((e) => {
    setSearchName(e?.target?.value);
  }, 500);
  const { jobDefinitions, loading: jobDefLoading } = useSelector(
    (state: RootState) => state.appReducer.jobDefinitions,
  );
  const {
    scheduledJobs,
    pagination,
    loading: scheduledJobLoading,
    pageable,
    scheduledJobStatus,
  } = useSelector((state: RootState) => state.appReducer.scheduledJobs);
  const [taskDefinition, setTaskDefinition] = useState<TaskDefinition | null>(
    null,
  );
  const [isModalVisible, setIsModalVisible] = useState<boolean>(false);
  const [scheduledJob, setScheduledJob] = useState<ScheduledJob | undefined>();

  const toggleSchedulerStatus = async () => {
    dispatch(toggleScheduledJobStatus(scheduledJobStatus!));
    await new Promise((resolve) => setTimeout(resolve, 50));
    dispatch(fetchScheduledJobStatus());
  };

  const handleUpsertScheduledJob = (values: UpsertType) => {
    const payload = {
      ...values,
      taskDefinition: taskDefinition,
    };
    dispatch(upsertScheduledJob(payload));
    setIsModalVisible(false);
  };

  const deleteJob = (job: ScheduledJob) => {
    dispatch(deleteScheduledJob(job._id));
  };

  const runJob = (job: ScheduledJob) => {
    dispatch(runScheduledJobManually(job._id));
  };

  const handleTableChange = (
    newPagination: PaginationProps,
    _filters: Record<string, (React.Key | boolean)[] | null>,
    sorter: SorterResult<ScheduledJob> | SorterResult<ScheduledJob>[],
  ) => {
    const sortParams: Record<string, 1 | -1> = {};

    if (Array.isArray(sorter)) {
      sorter.forEach((srt) => {
        if (srt.order) {
          sortParams[srt.field as string] = srt.order === "ascend" ? 1 : -1;
        }
      });
    } else if (sorter.order) {
      sortParams[sorter.field as string] = sorter.order === "ascend" ? 1 : -1;
    } else {
      sortParams["creationDate"] = -1;
    }

    dispatch(
      setPageable({
        page: newPagination.current,
        limit: newPagination.pageSize,
        sort: sortParams,
      }),
    );
  };

  useEffect(() => {
    if (searchName !== undefined)
      dispatch(
        setPageable({
          page: 1,
          filter: {
            name: {
              $regex: `${searchName}`,
              $options: "i",
            },
          },
        }),
      );
  }, [dispatch, searchName]);

  useEffect(() => {
    dispatch(fetchJobDefinitions());
    dispatch(fetchScheduledJobStatus());
  }, [dispatch]);

  useEffect(() => {
    dispatch(fetchScheduledJobs(pageable));
  }, [pageable, dispatch]);

  const columns: TableProps<ScheduledJob>["columns"] = [
    {
      title: () => (
        <Input placeholder="Name" onChange={searchNameDebounced}></Input>
      ),
      dataIndex: "name",
      responsive: ["sm"],
      width: "20%",
      key: "name",
      render: (name?: string) => name || "N/A",
    },
    {
      title: "Payload",
      width: "40%",
      dataIndex: "payload",
      key: "payload",
      render: (_, record) => {
        const payload = getPayloadFromScheduledJob(record);
        if (payload) {
          if (typeof payload === "string") {
            return (
              <a href={payload} target="_blank">
                {payload}{" "}
              </a>
            );
          } else if ("type" in payload) {
            return <Tag>{payload.type}</Tag>;
          }
        }
      },
    },
    {
      title: "Created",
      responsive: ["sm"],
      dataIndex: "creationDate",
      key: "creationDate",
      sorter: true,
      render: (date: string) =>
        dayjs(new Date(date)).format("DD/MM/YYYY HH:mm:ss"),
    },
    {
      title: "Next",
      responsive: ["sm"],
      dataIndex: "nextExecution",
      key: "nextExecution",
      sorter: true,
      render: (date?: string) =>
        date ? dayjs(new Date(date)).format("DD/MM/YYYY HH:mm:ss") : "N/A",
    },
    {
      title: "Cron",
      dataIndex: "cronExpr",
      key: "cronExpr",
    },
    {
      title: "Action",
      key: "action",
      align: "center",
      render: (_, record) => (
        <>
          <Popconfirm
            key={`${record._id}-delete`}
            placement="left"
            title="Delete the scheduled job"
            description="Are you sure to delete this scheduled job?"
            onConfirm={() => deleteJob(record)}
            okText="Yes"
            cancelText="No"
          >
            <Button
              disabled={!token}
              type="link"
              shape="default"
              danger
              icon={<DeleteOutlined />}
            />
          </Popconfirm>
          <Popconfirm
            key={`${record._id}-run`}
            placement="right"
            title="Run the scheduled job"
            description="Are you sure to run this scheduled job?"
            onConfirm={() => runJob(record)}
            okText="Yes"
            cancelText="No"
          >
            <Button
              disabled={!token}
              type="link"
              shape="default"
              icon={<ApiOutlined />}
            />
          </Popconfirm>
          <Button
            key={`${record._id}-edit`}
            disabled={!token}
            type="link"
            shape="default"
            icon={<EditOutlined />}
            onClick={(e) => {
              e.preventDefault();
              setScheduledJob(record);
              setIsModalVisible(true);
            }}
          />
        </>
      ),
    },
  ];

  return (
    <>
      <Flex justify="space-between">
        <h2>Scheduled Jobs</h2>
        {token && (
          <Space>
            {scheduledJobStatus && (
              <Button
                onClick={toggleSchedulerStatus}
                size="large"
                color="default"
                variant="dashed"
                icon={
                  scheduledJobStatus.status === "paused" ? (
                    <PlaySquareOutlined />
                  ) : (
                    <PauseOutlined />
                  )
                }
              >
                {isMobile
                  ? ""
                  : scheduledJobStatus.status === "paused"
                    ? "Start scheduler"
                    : "Pause scheduler"}
              </Button>
            )}
            <Button
              onClick={() => setIsModalVisible(true)}
              size="large"
              color="default"
              variant="dashed"
              icon={<PlusOutlined />}
            >
              {isMobile ? "" : "New Scheduled Job"}
            </Button>
            <Button
              onClick={() =>
                handleTableChange(
                  { current: 1 },
                  {},
                  {
                    field: "creationDate",
                    order: "descend",
                  },
                )
              }
              size="large"
              color="default"
              variant="dashed"
              icon={<SyncOutlined />}
            >
              {isMobile ? "" : "Refresh"}
            </Button>
          </Space>
        )}
      </Flex>
      <Table
        pagination={pagination}
        scroll={{ x: "max-content" }}
        loading={jobDefLoading || scheduledJobLoading}
        bordered
        dataSource={scheduledJobs}
        columns={columns}
        onChange={handleTableChange}
        rowKey="_id"
      />
      <UpsertScheduledJob
        key="upsert-scheduled-job"
        jobDefinitions={jobDefinitions}
        onFinish={handleUpsertScheduledJob}
        taskDefinition={taskDefinition}
        setTaskDefinition={setTaskDefinition}
        isModalVisible={isModalVisible}
        setIsModalVisible={setIsModalVisible}
        scheduledJob={scheduledJob}
      />
    </>
  );
};

export default ScheduledJobsTable;
