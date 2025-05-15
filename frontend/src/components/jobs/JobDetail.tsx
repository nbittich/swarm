import { colorForStatus, Job } from "@swarm/models/domain";
import { Descriptions, Tag, Typography } from "antd";
import dayjs from "dayjs";
import { Link } from "react-router-dom";
const { Text } = Typography;
const JobDetail = ({ job }: { job: Job }) => {
    return (<>{
        job && <Descriptions key={job._id as string}
            bordered
            column={1}
        >
            <Descriptions.Item styles={{ label: { width: '10vw', fontWeight: "bold" } }} label="_id" key={crypto.randomUUID()}>
                {job._id}
            </Descriptions.Item>
            <Descriptions.Item styles={{ label: { width: '10vw', fontWeight: "bold" } }} label="Name" key={crypto.randomUUID()}>
                {job.name}
            </Descriptions.Item>

            {job.targetUrl && <Descriptions.Item styles={{ label: { width: '10vw', fontWeight: "bold" } }} label="URL" key={crypto.randomUUID()}>
                <Link to={job.targetUrl}>{job.targetUrl}</Link>
            </Descriptions.Item>}
            <Descriptions.Item styles={{ label: { width: '10vw', fontWeight: "bold" } }} label="Created" key={crypto.randomUUID()}>
                {dayjs(new Date(job.creationDate)).format('DD/MM/YYYY HH:mm:ss')}
            </Descriptions.Item>
            {job.modifiedDate && <Descriptions.Item styles={{ label: { width: '10vw', fontWeight: "bold" } }} label="Modified" key={crypto.randomUUID()}>
                {dayjs(new Date(job.modifiedDate)).format('DD/MM/YYYY HH:mm:ss')}
            </Descriptions.Item>}
            <Descriptions.Item styles={{ label: { width: '10vw', fontWeight: "bold" } }} label="Directory" key={crypto.randomUUID()}>
                {job.rootDir}
            </Descriptions.Item>
            <Descriptions.Item styles={{ label: { width: '10vw', fontWeight: "bold" } }} label="Status" key={crypto.randomUUID()}>
                <Tag color={colorForStatus(job.status)} >
                    <Text style={{ whiteSpace: 'normal', wordBreak: 'break-word' }}>{job.status.type}
                        {job.status.type === "failed" && job.status.value && `: ${job.status.value.join(", ")}`}</Text>
                </Tag>
            </Descriptions.Item>
        </Descriptions>
    }</>)

}

export default JobDetail;
