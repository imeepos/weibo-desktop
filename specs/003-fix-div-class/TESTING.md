# 003-fix-div-class 测试指南

## 快速测试

### 1. 编译验证
```bash
cd /workspace/desktop/playwright
pnpm run build
```
✅ 预期: 编译成功,无错误

### 2. 生成二维码测试
```bash
node dist/weibo-login.js generate
```
✅ 预期输出:
```json
{
  "session_id": "qr_1733408123_abc123",
  "qr_image": "base64编码的图片数据",
  "expires_in": 180
}
```

### 3. 检查状态测试

#### 3.1 Pending状态
```bash
# 立即检查(二维码未扫描)
node dist/weibo-login.js check <session_id>
```
✅ 预期输出:
```json
{"status": "pending"}
```

#### 3.2 过期刷新测试
```bash
# 等待3分钟后检查
node dist/weibo-login.js check <session_id>
```
✅ 预期输出(自动刷新):
```json
{
  "status": "pending",
  "qr_refreshed": true,
  "qr_image": "新的base64编码图片数据"
}
```

#### 3.3 扫码状态测试
```bash
# 扫码但不确认后检查
node dist/weibo-login.js check <session_id>
```
✅ 预期输出:
```json
{"status": "scanned"}
```

#### 3.4 登录成功测试
```bash
# 扫码并确认后检查
node dist/weibo-login.js check <session_id>
```
✅ 预期输出:
```json
{
  "status": "confirmed",
  "cookies": {
    "SUB": "xxx",
    "SUBP": "yyy",
    ...
  },
  "uid": "5286244471",
  "screen_name": "用户昵称"
}
```

### 4. Cookie验证测试
```bash
# 使用获取到的cookies验证
node dist/validate-cookies.js '{"SUB":"xxx","SUBP":"yyy"}'
```
✅ 预期输出(有效):
```json
{
  "valid": true,
  "uid": "5286244471",
  "screen_name": "用户昵称"
}
```

✅ 预期输出(无效):
```json
{
  "valid": false,
  "error": "Invalid cookies or missing uid"
}
```

## 完整流程测试

### 步骤1: 生成二维码
```bash
cd /workspace/desktop/playwright
RESPONSE=$(node dist/weibo-login.js generate)
echo $RESPONSE | jq .

# 提取session_id
SESSION_ID=$(echo $RESPONSE | jq -r .session_id)
echo "Session ID: $SESSION_ID"

# 保存二维码图片用于扫描
echo $RESPONSE | jq -r .qr_image | base64 -d > /tmp/qrcode.png
echo "QR code saved to /tmp/qrcode.png"
```

### 步骤2: 轮询检查状态
```bash
# 每3秒检查一次
while true; do
  STATUS=$(node dist/weibo-login.js check $SESSION_ID)
  echo "$(date): $STATUS"

  # 检查是否刷新了二维码
  if echo $STATUS | jq -e '.qr_refreshed' > /dev/null; then
    echo "QR code refreshed! Updating image..."
    echo $STATUS | jq -r .qr_image | base64 -d > /tmp/qrcode.png
  fi

  # 检查是否登录成功
  if echo $STATUS | jq -e '.status == "confirmed"' > /dev/null; then
    echo "Login confirmed!"
    echo $STATUS | jq .
    break
  fi

  sleep 3
done
```

### 步骤3: 验证Cookie
```bash
# 从上一步的输出提取cookies
COOKIES=$(echo $STATUS | jq -c .cookies)
echo "Validating cookies..."

node dist/validate-cookies.js "$COOKIES"
```

## 调试技巧

### 查看页面快照
过期检测失败时,脚本会保存HTML快照:
```bash
cat /workspace/desktop/playwright/.sessions/debug-page.html
```

### 查看浏览器日志
启用详细日志:
```bash
DEBUG=pw:api node dist/weibo-login.js generate
```

### 查看选择器匹配
在浏览器中测试选择器:
```javascript
// 过期提示
document.querySelector('.absolute.top-28')?.textContent
// 预期: "该二维码已过期，请重新扫描"

// 刷新按钮
document.querySelector('.absolute.top-36 a')?.textContent
// 预期: "点击刷新"

// 扫码成功
document.querySelector('div:has-text("成功扫描")')?.textContent
// 预期: "成功扫描，请在手机点击确认以登录"
```

## 常见问题

### Q: 编译失败
```bash
# 清理并重新安装依赖
rm -rf node_modules dist
pnpm install
pnpm run build
```

### Q: 选择器找不到元素
检查微博页面是否更新了class名称:
1. 打开 https://passport.weibo.com/sso/signin
2. 点击"扫码登录"
3. 等待过期
4. 检查实际的class名称

### Q: Cookie验证失败
可能原因:
1. Cookie已过期
2. VIP API返回格式变化
3. 网络问题

调试方法:
```bash
# 直接测试VIP API
curl 'https://vip.weibo.com/aj/vipcenter/user' \
  -H 'Cookie: SUB=xxx; SUBP=yyy' \
  -H 'Referer: https://vip.weibo.com/home' | jq .
```

## 性能测试

### 测量二维码生成时间
```bash
time node dist/weibo-login.js generate
```
✅ 目标: <5秒

### 测量状态检查时间
```bash
time node dist/weibo-login.js check <session_id>
```
✅ 目标: <3秒

### 测量Cookie验证时间
```bash
time node dist/validate-cookies.js '{"SUB":"xxx"}'
```
✅ 目标: <2秒

## 集成测试

在Tauri环境中测试完整流程:
```bash
cd /workspace/desktop
cargo test --test integration_test -- --nocapture
```

## 验收标准

所有测试通过即可认为修复成功:
- [x] ✅ 编译无错误
- [ ] ⏳ 生成二维码成功
- [ ] ⏳ 过期自动刷新返回新二维码
- [ ] ⏳ 扫码状态检测正确
- [ ] ⏳ 登录成功获取Cookie
- [ ] ⏳ Cookie验证使用VIP API成功
- [ ] ⏳ uid正确提取

完成以上测试后,即可合并到主分支。
