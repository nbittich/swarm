
import MarkdownViewer from '../markdown/MarkdownViewer';

const gallery =
    ['/pages/home/index.png',
        '/pages/home/jobs.png',
        '/pages/home/schedule.png',
        '/pages/home/tasks.png',
        '/pages/home/subtasks.png',
        '/pages/home/swarm.png',
    ];
const Home = () => {
    return (
        <MarkdownViewer filePath="/pages/home/index.md" gallery={[]} />
    );
};

export default Home;
