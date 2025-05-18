import { getPayloadFromScheduledJob, JobDefinition, ScheduledJob, statusOptions, TaskDefinition } from "@swarm/models/domain";
import { Button, Flex, Form, Input, Modal, Select, Space } from "antd";
import cron from "cron-validate";
import { useEffect } from "react";

const { Option } = Select;

export type UpsertType = {
    __id?: string,
    jobName?: string,
    definitionId: string,
    cronExpr: string,
    targetUrl?: string,
    status?: string,
};



const UpsertScheduledJob = ({ scheduledJob, setIsModalVisible, isModalVisible, onFinish, jobDefinitions, taskDefinition, setTaskDefinition }: {
    jobDefinitions: JobDefinition[], onFinish: (values: UpsertType) => Promise<void>
    taskDefinition: TaskDefinition | null, setTaskDefinition: (def: TaskDefinition | null) => void, scheduledJob?: ScheduledJob,
    isModalVisible: boolean | undefined, setIsModalVisible: (v: boolean) => void,
}) => {

    const [form] = Form.useForm<UpsertType>();
    const toggleModal = (value: boolean) => {
        form.resetFields();
        setTaskDefinition(null);
        setIsModalVisible(value);
    };
    const onFinishAndCleanup = (values: UpsertType) => {
        onFinish(values);
        toggleModal(false);
    }
    const handleJobDefinitionChange = (id: string) => {
        const selectedJD = jobDefinitions.find((jd) => jd.id === id);

        if (selectedJD && selectedJD.tasks.length > 0) {
            const firstTask = selectedJD.tasks[0];

            setTaskDefinition(firstTask);
        }
    };

    useEffect(() => {
        if (scheduledJob) {
            const selectedJD = jobDefinitions.find((jd) => jd.id === scheduledJob.definitionId);

            const payload = getPayloadFromScheduledJob(scheduledJob);
            if (selectedJD && selectedJD.tasks.length) {
                setTaskDefinition(selectedJD.tasks[0]);
            }

            form.setFieldsValue({
                __id: scheduledJob._id,
                jobName: scheduledJob.name,
                definitionId: scheduledJob.definitionId,
                cronExpr: scheduledJob.cronExpr,
                targetUrl: payload && typeof payload === 'string' ? payload : undefined,
                status: payload && typeof payload === 'object' && 'type' in payload ? payload.type : undefined,

            });

        } else {
            form.resetFields();
            setTaskDefinition(null);
        }
    }, [form, jobDefinitions, scheduledJob, setTaskDefinition]);
    return (

        <Modal
            key='edit-scheduled-job-modal'
            title={"Edit Scheduled Job"}
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
            <Form form={form} onFinish={onFinishAndCleanup} layout="vertical">
                <Form.Item
                    name="__id"
                    hidden
                >
                    <Input />
                </Form.Item>
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

    );
}

export default UpsertScheduledJob;
