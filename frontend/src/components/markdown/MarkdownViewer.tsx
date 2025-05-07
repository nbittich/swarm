import { useState, useEffect } from 'react';
import { Card, Col, Flex, Image, Space, } from 'antd';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

const MarkdownViewer = ({ filePath, gallery }: { filePath: string, gallery?: string[] }) => {
    const [markdown, setMarkdown] = useState('');

    useEffect(() => {
        fetch(filePath)
            .then((response) => {
                if (!response.ok) {
                    throw new Error('Failed to fetch Markdown file');
                }
                return response.text();
            })
            .then((text) => setMarkdown(text))
            .catch((error) => console.error('Error loading Markdown:', error));
    }, [filePath]);

    return (
        <Card>

            <Flex justify='center' align='center' vertical>
                <Col span={18}>
                    <Space>
                        {gallery && gallery.map(pic => <Image key={pic} preview={true} loading='lazy' src={pic}

                        />)}
                    </Space>

                </Col>
                <Col span={18}>
                    <ReactMarkdown remarkPlugins={[remarkGfm]}>{markdown}</ReactMarkdown>
                </Col>


            </Flex>

        </Card>

    );
};

export default MarkdownViewer;
