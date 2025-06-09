import { useEffect, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom"; // For extracting path variables
import {
  Table,
  Tag,
  Button,
  TableProps,
  Flex,
  Space,
  Typography,
  Select,
} from "antd";
import {
  colorForStatus,
  Status,
  SubTask,
  truncate,
} from "@swarm/models/domain";
import { ArrowLeftOutlined, DownloadOutlined } from "@ant-design/icons";
import { useDispatch, useSelector } from "react-redux";
import { AppDispatch, RootState } from "@swarm/states/Store";
import { fetchSubTasks, reset } from "@swarm/states/SuBTaskSlice";
import { download } from "@swarm/states/file/Api";
import dayjs from "dayjs";
import Link from "antd/es/typography/Link";
import { useAuth } from "@swarm/auth/authContextHook";
import { useIsMobile } from "@swarm/hooks/is-mobile";

const { Text } = Typography;
const SubTasksTable: React.FC = () => {
  const isMobile = useIsMobile();
  const { token } = useAuth();
  const navigate = useNavigate();
  const {
    id: jobId,
    taskId,
    taskName,
  } = useParams<{ id: string; taskId: string; taskName: string }>();
  const dispatch = useDispatch<AppDispatch>();

  const { data, loading } = useSelector(
    (state: RootState) => state.appReducer.subTasks,
  );

  const [currentPages, setCurrentPages] = useState<(string | undefined)[]>([]); // for the prev mechanism
  const [pageSize, setPageSize] = useState(10);
  const pageSizeRef = useRef(pageSize);
  useEffect(() => {
    dispatch(reset());
    if (jobId && taskId) {
      dispatch(
        fetchSubTasks({
          jobId,
          taskId,
          next: null,
          pageSize: pageSizeRef.current,
        }),
      );
    }
  }, [dispatch, pageSizeRef, jobId, taskId]);
  const reload = (newPageSize?: number) => {
    if (jobId && taskId && data.next) {
      setCurrentPages([]);
      dispatch(
        fetchSubTasks({
          jobId,
          taskId,
          next: null,
          pageSize: newPageSize || pageSize,
        }),
      );
    }
  };
  const handleNext = () => {
    if (jobId && taskId && data.next) {
      setCurrentPages([...currentPages, data.current]);
      dispatch(fetchSubTasks({ jobId, taskId, next: data.next, pageSize }));
    }
  };

  const handleBack = () => {
    const prev = currentPages.pop();
    if (jobId && taskId) {
      dispatch(fetchSubTasks({ jobId, taskId, next: prev, pageSize }));
    }
  };

  const columns: TableProps<SubTask>["columns"] = [
    {
      title: "Base url",
      key: "baseUrl",
      render: (_, record) => {
        if (
          record.result &&
          (record.result.type === "nTriple" ||
            record.result.type === "diff" ||
            record.result?.type == "scrapeUrl")
        ) {
          return (
            <a href={record.result.value.baseUrl} target="_blank">
              {record.result.value.baseUrl}{" "}
            </a>
          );
        } else {
          return <>N/A</>;
        }
      },
    },
    {
      title: "Created",
      responsive: ["sm"],
      dataIndex: "creationDate",
      key: "creationDate",
      render: (date: string) =>
        dayjs(new Date(date)).format("DD/MM/YYYY HH:mm:ss"),
    },

    {
      title: "Status",
      dataIndex: "status",
      key: "status",
      render: (status: Status) => {
        return (
          <Tag color={colorForStatus(status)}>
            <Text>
              {status.type}
              {status.type === "failed" &&
                status.value &&
                `: ${truncate(status.value.join(", "), 32)}`}
            </Text>
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
              component = (
                <Tag>
                  <Link
                    disabled={!token || !res.value.path}
                    onClick={async () => await download(jobId, res.value.path)}
                  >
                    <DownloadOutlined />
                    file.html
                  </Link>
                </Tag>
              );
              break;
            case "diff":
              component = (
                <Space>
                  {res.value.toRemovePath && (
                    <Link
                      disabled={!token || !res.value.toRemovePath}
                      onClick={async () =>
                        await download(jobId, res.value.toRemovePath)
                      }
                    >
                      <DownloadOutlined />
                      to-remove.ttl
                    </Link>
                  )}
                  {res.value.intersectPath && (
                    <Link
                      disabled={!token || !res.value.intersectPath}
                      onClick={async () =>
                        await download(jobId, res.value.intersectPath)
                      }
                    >
                      <DownloadOutlined />
                      intersect.ttl
                    </Link>
                  )}
                  {res.value.newInsertPath && (
                    <Link
                      disabled={!token || !res.value.newInsertPath}
                      onClick={async () =>
                        await download(jobId, res.value.newInsertPath)
                      }
                    >
                      <DownloadOutlined />
                      new-inserts.ttl
                    </Link>
                  )}
                </Space>
              );
              break;
            case "nTriple":
              component = (
                <Tag>
                  <Link
                    disabled={!token || !res.value.path}
                    onClick={async () => await download(jobId, res.value.path)}
                  >
                    <DownloadOutlined />
                    result.ttl
                  </Link>
                </Tag>
              );
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
          <Button
            onClick={() => navigate(`/jobs/${jobId}/tasks`)}
            icon={<ArrowLeftOutlined />}
            size="large"
            color="default"
            variant="dashed"
          >
            {!isMobile && "Back"}
          </Button>
        </Flex>
        <Table
          bordered
          scroll={{ x: "max-content" }}
          dataSource={data.content}
          columns={columns}
          rowKey="_id"
          loading={loading}
          pagination={false}
        ></Table>
        {data.content && (
          <Flex justify="end">
            <Space>
              <Select
                defaultValue={10}
                onChange={(value) => {
                  setPageSize(value);
                  reload(value);
                }}
              >
                {[5, 10, 20, 50, 100].map((size) => (
                  <Select.Option key={size} value={size}>
                    {size}
                  </Select.Option>
                ))}
              </Select>
              <Button disabled={!currentPages.length} onClick={handleBack}>
                Prev
              </Button>
              <Button disabled={!data?.next} onClick={handleNext}>
                Next
              </Button>
            </Space>
          </Flex>
        )}
      </Flex>
    </>
  );
};

export default SubTasksTable;
