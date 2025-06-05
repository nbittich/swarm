import React, { useEffect, useState } from "react";
import { Alert, Button, Flex, Space, Table, TableProps, Tag } from "antd";
import { useNavigate } from "react-router-dom"; // for accessing the dynamic route parameter
import {
  Batch,
  BatchStatus,
  colorForBatchStatus,
  labelForBatchStatus,
} from "@swarm/models/domain";
import { ArrowLeftOutlined } from "@ant-design/icons";
import { useIsMobile } from "@swarm/hooks/is-mobile";
import { useDispatch, useSelector } from "react-redux";
import { AppDispatch, RootState } from "@swarm/states/Store";
import { fetchSearchBatches } from "@swarm/states/SearchSlice";

const BatchTable: React.FC = () => {
  const isMobile = useIsMobile();
  const dispatch = useDispatch<AppDispatch>();
  const navigate = useNavigate();
  const [statuses, setStatuses] = useState<BatchStatus[] | undefined>();
  const batches = useSelector(
    (state: RootState) => state.appReducer.search.batches,
  );
  const loading = useSelector(
    (state: RootState) => state.appReducer.search.loading,
  );

  const error = useSelector(
    (state: RootState) => state.appReducer.search.error,
  );
  useEffect(() => {
    dispatch(fetchSearchBatches(statuses));
  }, [dispatch, statuses]);

  const columns: TableProps<Batch>["columns"] = [
    {
      title: "Progress",
      dataIndex: "progress",
      key: "progress",
      render: (_, record) => {
        return (
          <>
            <Tag>{record.progress.percentage}%</Tag>
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
        <Space>
          <Button
            onClick={() => navigate("/")}
            icon={<ArrowLeftOutlined />}
            size="large"
            color="default"
            variant="dashed"
          >
            {!isMobile && "Back"}
          </Button>
        </Space>
      </Flex>

      <Table
        bordered
        dataSource={batches}
        columns={columns}
        rowKey="uid"
        loading={loading}
        scroll={{ x: "max-content" }}
      />
    </Flex>
  );
};

export default BatchTable;
