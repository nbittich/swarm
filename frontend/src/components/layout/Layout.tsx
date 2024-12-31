import { Link, Outlet, useLocation, } from "react-router-dom";
import {
  LogoutOutlined,
  MenuFoldOutlined, MenuUnfoldOutlined,
  ThunderboltOutlined,
  ConsoleSqlOutlined,
  LoginOutlined,
  CalendarOutlined,
} from "@ant-design/icons";
import { Button, ConfigProvider, Image, Layout, Menu, MenuProps, Switch, theme } from "antd";
import { useState } from "react";
import MenuItem from "antd/es/menu/MenuItem";
import { useAuth } from "@swarm/auth/authContextHook";
// import frBE from 'antd/lib/locale/fr_BE';
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
    children,
    label,
  } as MenuItem;
}

const { Header, Sider, Content, } = Layout;
export default function MainLayout() {
  const [collapsed, setCollapsed] = useState(false);

  const { token, userClaims: _ } = useAuth();
  const location = useLocation();

  const [darkMode, setDarkMode] = useState(false);
  const darkToken = theme.getDesignToken({ algorithm: theme.darkAlgorithm });
  const defaultToken = theme.useToken().token;
  const backgroundColor = darkMode ? darkToken.colorBgContainer : defaultToken.colorBgContainer;

  const currenTheme = () => darkMode ? "dark" : 'light'; // fixme this should go to store
  const handleThemeToggle = (checked: boolean) => {
    setDarkMode(checked);
  };
  const items: MenuItem[] = [
  ];

  if (token) {
    items.push(getItem((<Link to="/jobs">Jobs</Link>), '/jobs', <ThunderboltOutlined />));
    items.push(getItem((<Link to="/scheduled-jobs">Scheduled Jobs</Link>), '/scheduled-jobs', <CalendarOutlined />));
    items.push(getItem((<Link to="/yasgui">Sparql</Link>), '/sparql', <ConsoleSqlOutlined />));
    items.push(getItem((<Link to="/logout">Logout</Link>), '/logout', <LogoutOutlined />));
  } else {
    items.push(getItem((<Link to="/login">Login</Link>), '/', <LoginOutlined />));
    items.push(getItem((<Link to="/yasgui">Sparql</Link>), '/sparql', <ConsoleSqlOutlined />));
  }



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
            siderBg: darkToken.colorBgBase,
            headerBg: darkMode ? darkToken["cyan-1"] : defaultToken["cyan-7"]
          },
          Menu: {
            darkItemBg: darkToken.colorFillQuaternary,
          },
        }
      }}>
      <Layout style={{ height: '100vh', width: '100vw' }}>
        <Sider width={0} collapsedWidth={80} theme={currenTheme()} trigger={null}
          collapsible collapsed={collapsed}>
          <Menu
            style={{ height: '100%' }}
            mode="inline"
            theme={currenTheme()}
            defaultSelectedKeys={[location.pathname]}
            items={items}
          >
          </Menu>
        </Sider>
        <Layout>
          <Header style={{
            padding: 0,
            justifyContent: 'space-between', display: 'flex', alignItems: "center"
          }}>
            <Button
              type="text"
              icon={collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
              onClick={() => setCollapsed(!collapsed)}
              style={{
                fontSize: '16px',
                width: 64,
                height: 64,
                border: 0,
                borderRadius: 0
              }}
            />
            <div>
              <Image src="/favicon.svg" width={24} height={24}
                style={{
                  display: "inline-block",
                  verticalAlign: "middle", // Aligns the image vertically in text
                }} />
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
            <Outlet />
          </Content>

        </Layout>
      </Layout>
    </ConfigProvider >
  );
}
