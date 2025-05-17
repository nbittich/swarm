import { JobDefinition, statusOptions, TaskDefinition } from "@swarm/models/domain";
import { Form, FormInstance, Input, Select } from "antd";
import cron from "cron-validate";

const { Option } = Select;

type UpsertType = {
    jobName?: string,
    definitionId: string,
    cronExpr: string,
    targetUrl?: string,
    status?: string,
};

const UpsertScheduledJob = ({ form, onFinish, jobDefinitions, taskDefinition, setTaskDefinition }: {
    form: FormInstance<UpsertType>, jobDefinitions: JobDefinition[], onFinish: (values: UpsertType) => Promise<void>
    taskDefinition: TaskDefinition | null, setTaskDefinition: (def: TaskDefinition) => void
}) => {

    const handleJobDefinitionChange = (id: string) => {
        const selectedJD = jobDefinitions.find((jd) => jd.id === id);

        if (selectedJD && selectedJD.tasks.length > 0) {
            const firstTask = selectedJD.tasks[0];

            setTaskDefinition(firstTask);
        }
    };
    return (<Form form={form} onFinish={onFinish} layout="vertical">
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
    </Form>);
}

export default UpsertScheduledJob;
