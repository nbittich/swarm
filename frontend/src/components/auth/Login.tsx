import { useAuth } from "@swarm/auth/authContextHook";
import { useState } from "react";
import axios from "axios";
import { Form, Input, Button, message, Card, } from 'antd';
import { useNavigate } from "react-router-dom";
export default function Login() {
  const { setToken } = useAuth();
  const navigate = useNavigate();
  const [loading, setLoading] = useState(false);
  const handleLogin = async (form: { username: string; password: string }) => {
    try {
      const response = await axios.post('/api/login', { username: form.username, password: form.password });
      if (response.status == 200) {
        const jwt = response.data?.access_token;
        if (!jwt) {
          console.error("response doesn't contain a jwt", response.data);
          throw "no jwt in response";
        }
        setToken(jwt);
        // message.success('Login successful!');
        navigate("/", { replace: true });

      } else {
        message.error(`Could not connect: ${response.status}`);
      }

    } catch (err) {
      if (axios.isCancel(err)) {
        console.log('Request canceled', err.message);
      } else {
        message.error('Error fetching data');
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <Card title="Login">
      <Form
        name="login"
        layout="vertical"
        onFinish={handleLogin}
      >
        <Form.Item
          label="Username"
          name="username"
          rules={[{ required: true, message: 'Please input your username!' }]}
        >
          <Input />
        </Form.Item>

        <Form.Item
          label="Password"
          name="password"
          rules={[{ required: true, message: 'Please input your password!' }]}
        >
          <Input.Password />
        </Form.Item>
        <Form.Item>
          <Button type="dashed" htmlType="submit" loading={loading} style={{ width: '100%' }}>
            Log in
          </Button>
        </Form.Item>
      </Form>
    </Card>
  )

}
