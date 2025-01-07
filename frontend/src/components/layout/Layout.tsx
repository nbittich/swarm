import { Outlet, useLocation, } from "react-router-dom";
import {
    LogoutOutlined,
    MenuFoldOutlined, MenuUnfoldOutlined,
    ThunderboltOutlined,
    ConsoleSqlOutlined,
    LoginOutlined,
    CalendarOutlined,
} from "@ant-design/icons";
import { App, Button, ConfigProvider, Flex, Image, Layout, Menu, MenuProps, Switch, theme } from "antd";
import { useEffect, useState } from "react";
import MenuItem from "antd/es/menu/MenuItem";
import { useAuth } from "@swarm/auth/authContextHook";
import { useDispatch, useSelector } from "react-redux";
import { RootState } from "@swarm/states/Store";
import { toggleTheme } from "@swarm/states/ThemeSlice";
import { gray, } from "@ant-design/colors";
import { useNavigate } from "react-router-dom";
/* import frBE from 'antd/lib/locale/fr_BE'; */
type MenuItem = Required<MenuProps>['items'][number];

function getItem(
    label: React.ReactNode,
    key: React.Key,
    icon?: React.ReactNode,
    children?: MenuItem[],
): MenuItem {
    return {
        key,
        icon,
        title: label,
        children,
        label,
    } as MenuItem;
}

const { Header, Sider, Content, } = Layout;
export default function MainLayout() {
    const dispatch = useDispatch();
    const [collapsed, setCollapsed] = useState(false);
    const { token, userClaims: _ } = useAuth();
    const location = useLocation();
    const navigate = useNavigate();
    const darkMode = useSelector((state: RootState) => state.theme.darkMode);
    const currenTheme = () => darkMode ? "dark" : 'light';
    const darkToken = theme.getDesignToken({ algorithm: theme.darkAlgorithm });
    const defaultToken = theme.useToken().token;
    const backgroundColor = darkMode ? darkToken.colorBgContainer : defaultToken.colorBgContainer;
    const handleThemeToggle = (checked: boolean) => {
        dispatch(toggleTheme(checked));
    };

    const items: MenuItem[] = [
    ];

    if (token) {
        items.push(getItem((<a onClick={() => handleNavigation("/jobs")}>Jobs</a>), '/jobs', <ThunderboltOutlined />));
        items.push(getItem((<a onClick={() => handleNavigation("/scheduled-jobs")}>Scheduled Jobs</a>), '/scheduled-jobs', <CalendarOutlined />));
        items.push(getItem((<a onClick={() => handleNavigation("/yasgui")}>Sparql</a>), '/yasgui', <ConsoleSqlOutlined />));
        items.push({ type: 'divider' });
        items.push(getItem((<a onClick={() => handleNavigation("/logout")}>Logout</a>), '', <LogoutOutlined />));
    } else {
        items.push(getItem((<a onClick={() => handleNavigation("/yasgui")}>Sparql</a>), '/yasgui', <ConsoleSqlOutlined />));
        items.push({ type: 'divider' });
        items.push(getItem((<a onClick={() => handleNavigation("/login")}>Login</a>), '', <LoginOutlined />));
    }



    useEffect(() => {
        dispatch(toggleTheme(window.matchMedia('(prefers-color-scheme: dark)').matches));
    }, [dispatch]);

    const handleNavigation = (route: string) => {
        if (location.pathname != route) {
            dispatch({ type: route });
            navigate(route);
        }
    };
    return (
        <ConfigProvider
            // locale={frBE}
            theme={{
                token: {
                    borderRadiusOuter: 0,
                    borderRadius: 0
                },
                algorithm: darkMode ? theme.darkAlgorithm : theme.defaultAlgorithm, components: {
                    Layout: {
                        headerBg: darkMode ? darkToken.colorBgBlur : defaultToken.colorBgBase,
                        headerPadding: 0,
                    },
                    Menu: {
                        darkItemBg: darkToken.colorBgMask,
                    },

                }
            }}>
            <Layout style={{ height: '100vh', width: '100vw' }}>
                <Sider collapsedWidth={0} theme={currenTheme()} trigger={null}
                    style={{ display: "flex", flexDirection: 'column', }}
                    collapsible collapsed={collapsed}>
                    <Flex vertical style={{ height: '100vh' }}
                    >
                        <Menu
                            style={{
                                flexGrow: 1,
                                borderWidth: darkToken ? 0.1 : undefined,
                                borderRightStyle: 'dotted',
                                borderColor: darkMode ? gray[6] : gray[1],
                            }}
                            theme={currenTheme()}
                            selectedKeys={[location.pathname]}
                            items={items.slice(0, -2)}
                        />
                        <Menu

                            style={{
                                borderWidth: darkToken ? 0.1 : undefined,
                                borderRightStyle: 'dotted',
                                borderColor: darkMode ? gray[6] : gray[1],
                            }}
                            theme={currenTheme()}
                            selectedKeys={[]}
                            defaultSelectedKeys={[]}
                            items={items.slice(-2)}
                        />
                    </Flex>

                </Sider>
                <Layout>
                    <Header style={{
                        height: 48,
                        width: '100%',
                        justifyContent: 'space-between', display: 'flex', alignItems: "center"
                    }}>
                        <Button
                            type="text"
                            icon={collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
                            onClick={() => setCollapsed(!collapsed)}
                            style={{
                                fontSize: '16px',
                                width: 32,
                                height: 32,
                                border: 0,
                                marginLeft: "15px",
                                borderRadius: 0
                            }}
                        />
                        <div>
                            <a onClick={() => handleNavigation("/")}><Image preview={false} src="/favicon.svg" width={24} height={24}
                                style={{
                                    display: "inline-block",
                                    padding: 0,
                                    margin: 0,
                                    verticalAlign: "middle",
                                }} /></a>
                        </div>

                        <Switch
                            checked={darkMode}
                            style={{ padding: 0, marginRight: 10 }}
                            onChange={handleThemeToggle}
                            checkedChildren=""
                            unCheckedChildren=""
                        />
                    </Header>
                    <Content
                        style={{
                            backgroundColor,
                            margin: '24px 16px',
                            padding: 24,
                            minHeight: '238px',

                            overflow: 'auto',
                            display: 'flex',
                            flexDirection: 'column',
                        }}
                    >
                        <App>
                            <Outlet />
                        </App>

                    </Content>

                </Layout>
            </Layout>
        </ConfigProvider >
    );
}
