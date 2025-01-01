import { useEffect, } from "react";
import { useNavigate, useParams } from "react-router-dom"; // For extracting path variables
import { Table, Tag, Button, TableProps, Flex, Space, Pagination, } from "antd";
import { colorForStatus, Status, SubTask, } from "@swarm/models/domain";
import { ArrowLeftOutlined, DownloadOutlined, PlusOutlined } from "@ant-design/icons";
import { useDispatch, useSelector } from "react-redux";
import { AppDispatch, RootState } from "@swarm/states/Store";
import { fetchSubTasks, reset } from "@swarm/states/SuBTaskSlice";
import { download } from "@swarm/states/file/Api";

const SubTasksTable: React.FC = () => {
  const navigate = useNavigate();
  const { id: jobId, taskId, taskName } = useParams<{ id: string; taskId: string, taskName: string }>();
  const dispatch = useDispatch<AppDispatch>();

  const { data, loading, lastElementId } = useSelector((state: RootState) => state.subTasks);

  const pageSize = 50;
  useEffect(() => {
    dispatch(reset());
    if (jobId && taskId) {
      dispatch(fetchSubTasks({ jobId, taskId, lastElementId: null, pageSize }));
    }
  }, [dispatch, jobId, taskId, taskName]);
  const loadMore = () => {
    if (jobId && taskId) {
      dispatch(fetchSubTasks({ jobId, taskId, lastElementId, pageSize }));
    }
  };

  const columns: TableProps<SubTask>["columns"] = [
    {
      title: "Base url",
      key: "baseUrl",
      render: (_, record) => {
        if (record.result &&
          (record.result.type === "nTriple" ||
            record.result.type === "diff" ||
            record.result?.type == "scrapeUrl")) {
          return <a href={record.result.value.baseUrl} target="_blank">{record.result.value.baseUrl} </a>
        } else {
          return (<>
            N/A
          </>)

        }
      }
    },
    {
      title: "Creation Date",
      dataIndex: "creationDate",
      key: "creationDate",
      render: (date: string) => new Date(date).toLocaleString(),
    },

    {
      title: "Status",
      dataIndex: "status",
      key: "status",
      render: (status: Status) => {
        return (
          <Tag color={colorForStatus(status)}>
            {status.type.toUpperCase()}
            {status.type === "failed" && status.value && `: ${status.value.join(", ")}`}
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
              component = <Tag>
                <a onClick={async () => await download(jobId, res.value.path)}><DownloadOutlined />file.html</a>
              </Tag>;
              break;
            case "diff":
              component = <Space>
                {res.value.toRemovePath && <a onClick={async () => await download(jobId, res.value.toRemovePath)}><DownloadOutlined />to-remove.ttl</a>}
                {res.value.intersectPath && <a onClick={async () => await download(jobId, res.value.intersectPath)}><DownloadOutlined />intersect.ttl</a>}
                {res.value.newInsertPath && <a onClick={async () => await download(jobId, res.value.newInsertPath)}><DownloadOutlined />new-inserts.ttl</a>}
              </Space>;
              break;
            case "nTriple":
              component = <Tag>
                <a onClick={async () => await download(jobId, res.value.path)}><DownloadOutlined />result.ttl</a>
              </Tag>;
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
          <Space>
            <Button onClick={() => navigate(`/jobs/${jobId}/tasks`)} icon={<ArrowLeftOutlined />} size="large" color="default" variant="dashed">Back</Button>
            <Button disabled={loading || !lastElementId} onClick={loadMore} size="large" color="default" variant="dashed" icon={<PlusOutlined />}>
              Load {pageSize} more
            </Button>
          </Space>

        </Flex>
        <Table
          bordered
          dataSource={data}
          columns={columns}
          rowKey="_id"
          loading={loading}

        >
          <Pagination responsive pageSize={pageSize} />
        </Table>
      </Flex>

    </>

  );
};

export default SubTasksTable;
