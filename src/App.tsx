import { useState } from "react";

function App() {
  const [status, setStatus] = useState("等待扫码...");

  return (
    <div className="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100 flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-xl p-8 w-full max-w-md">
        <h1 className="text-2xl font-bold text-gray-800 mb-6 text-center">
          微博登录助手
        </h1>

        <div className="bg-gray-100 rounded-lg p-6 mb-6 aspect-square flex items-center justify-center">
          <p className="text-gray-500">二维码加载中...</p>
        </div>

        <div className="text-center">
          <p className="text-sm text-gray-600">{status}</p>
        </div>
      </div>
    </div>
  );
}

export default App;
