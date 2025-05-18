import { useState, useEffect } from 'react';
import { Card, Col, Flex, Image, Space, Typography, } from 'antd';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { useSelector } from 'react-redux';
import { RootState } from '@swarm/states/Store';
import { useIsMobile } from '@swarm/hooks/is-mobile';

const MarkdownViewer = ({ filePath, gallery }: { filePath: string, gallery?: string[] }) => {
    const [markdown, setMarkdown] = useState('');

    const isMobile = useIsMobile();
    const getDarkImg = (src: string | undefined) => {
        if (!src) {
            return src;
        }
        const parts = src.split('/');
        parts[parts.length - 1] = `dark-${parts[parts.length - 1]}`;
        return parts.join('/');
    }

    const darkMode = useSelector((state: RootState) => state.theme.darkMode);
    const MarkdownComp =
        () => (<ReactMarkdown
            components={{
                a: ({ href, children, ...props }) => <a target="_blank" href={href} {...props} >{children}</a>,
                p: (props) => <Typography.Paragraph {...props} />,
                img: ({ src, alt, title }) => (
                    <Image
                        src={darkMode ? getDarkImg(src) : src}
                        alt={alt}
                        title={title}
                        preview={false} />
                ),
            }}
            remarkPlugins={[remarkGfm]}>{markdown}</ReactMarkdown>);
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

        <Flex justify='center' align='center' vertical>
            <Col span={isMobile ? 24 : 18}>
                <Space>
                    {gallery && gallery.map(pic => <Image key={pic} preview={true} loading='lazy' src={pic}

                    />)}
                </Space>

            </Col>
            <Col span={isMobile ? 24 : 18}>
                {isMobile ? <MarkdownComp /> : <Card>
                    <MarkdownComp />
                </Card>}
            </Col>


        </Flex>


    );
};

export default MarkdownViewer;
