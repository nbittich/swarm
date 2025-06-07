import React, { useState } from "react";
import {
  Alert,
  Button,
  Flex,
  Row,
  Select,
  Space,
  Table,
  TableProps,
  Tag,
} from "antd";
import {
  Batch,
  BatchStatus,
  colorForBatchStatus,
  labelForBatchStatus,
} from "@swarm/models/domain";
import { SyncOutlined } from "@ant-design/icons";
import { useIsMobile } from "@swarm/hooks/is-mobile";
import { useDispatch, useSelector } from "react-redux";
import { AppDispatch, RootState } from "@swarm/states/Store";
import { fetchSearchBatches } from "@swarm/states/SearchSlice";
import dayjs from "dayjs";
import useMountEffect from "@swarm/hooks/useMountEffect";
const batchStatusOptions: BatchStatus[] = [
  "enqueued",
  "processing",
  "succeeded",
  "failed",
  "canceled",
];
const BatchTable: React.FC = () => {
  const isMobile = useIsMobile();
  const dispatch = useDispatch<AppDispatch>();
  const [statuses, setStatuses] = useState<BatchStatus[] | undefined>([
    "enqueued",
    "failed",
    "processing",
    "canceled",
    "succeeded",
  ]);
  const [currentPages, setCurrentPages] = useState<(number | undefined)[]>([]);
  const batches = useSelector(
    (state: RootState) => state.appReducer.search.batches,
  );
  const loading = useSelector(
    (state: RootState) => state.appReducer.search.loading,
  );

  const error = useSelector(
    (state: RootState) => state.appReducer.search.error,
  );

  useMountEffect(() => {
    dispatch(
      fetchSearchBatches({
        statuses: statuses,
        next: batches?.next,
      }),
    );
  });

  const handleNext = () => {
    if (batches?.next) {
      setCurrentPages([...currentPages, batches.current]);
      dispatch(
        fetchSearchBatches({
          statuses: statuses,
          next: batches.next,
        }),
      );
    }
  };

  const handleBack = () => {
    const prev = currentPages.pop();
    dispatch(
      fetchSearchBatches({
        statuses: statuses,
        next: prev,
      }),
    );
  };
  const columns: TableProps<Batch>["columns"] = [
    {
      title: "Index",
      dataIndex: "indexUids",
      key: "idx",
      render: (_, record) => {
        return <>{Object.keys(record.stats.indexUids)}</>;
      },
    },
    {
      title: "Start",
      dataIndex: "startedAt",
      key: "startedAt",
      render: (_, record) => {
        return (
          <>{dayjs(new Date(record.startedAt)).format("DD/MM/YYYY HH:mm:ss")}</>
        );
      },
    },
    {
      title: "Finish",
      dataIndex: "finishedAt",
      key: "finishedAt",
      render: (_, record) => {
        return (
          <>
            {record.finishedAt
              ? dayjs(new Date(record.finishedAt)).format("DD/MM/YYYY HH:mm:ss")
              : "N/A"}
          </>
        );
      },
    },
    {
      title: "Total tasks",
      dataIndex: "totalNbTasks",
      key: "totalNbTasks",
      render: (_, record) => {
        return <>{record.stats.totalNbTasks}</>;
      },
    },
    {
      title: "Types",
      dataIndex: "types",
      key: "types",
      render: (_, record) => {
        return <pre>{JSON.stringify(record.stats.types)} </pre>;
      },
    },
    {
      title: "Progress",
      dataIndex: "progress",
      key: "progress",
      render: (_, record) => {
        return (
          <>
            <Tag>{record.progress?.percentage || 100}%</Tag>
          </>
        );
      },
    },

    {
      title: "Status",
      dataIndex: "status",
      key: "status",
      render: (_, record) => {
        const { succeeded, failed, enqueued, canceled, processing } =
          record.stats.status;
        return (
          <>
            <Tag
              title={labelForBatchStatus(record.stats.status)}
              color={colorForBatchStatus(record.stats.status)}
            >
              {labelForBatchStatus(record.stats.status)}:
              {succeeded || failed || enqueued || canceled || processing || 0}
            </Tag>
          </>
        );
      },
    },
  ];

  return (
    <Flex vertical gap="middle">
      <Flex justify="space-between" wrap>
        <h2>Search Batches</h2>
        {error && (
          <Alert
            message="Error"
            description={error}
            type="error"
            showIcon
            closable
          />
        )}
        <Button
          onClick={() => setStatuses([...(statuses || [])])}
          size="large"
          color="default"
          variant="dashed"
          icon={<SyncOutlined />}
        >
          {!isMobile && "Refresh"}
        </Button>
      </Flex>
      <Row>
        <Select
          mode="multiple"
          allowClear
          style={{ width: "100%" }}
          placeholder="Select batch statuses"
          value={statuses}
          onChange={(value) => setStatuses(value as BatchStatus[])}
        >
          {batchStatusOptions.map((status) => (
            <Select.Option key={status} value={status}>
              {status}
            </Select.Option>
          ))}
        </Select>
      </Row>
      <Table
        bordered
        dataSource={batches?.batches || []}
        columns={columns}
        rowKey="uid"
        loading={loading}
        scroll={{ x: "max-content" }}
        pagination={false} // Disable built-in pagination
      ></Table>
      {batches && (
        <Flex justify="end">
          <Space>
            <Button disabled={!currentPages.length} onClick={handleBack}>
              Prev
            </Button>
            <Button disabled={!batches?.next} onClick={handleNext}>
              Next
            </Button>
          </Space>
        </Flex>
      )}
    </Flex>
  );
};

export default BatchTable;
