import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

// 运行 react 主入口文件，渲染app
ReactDOM.createRoot(document.getElementById("root")).render(
    <React.StrictMode>
        <App/>
    </React.StrictMode>,
);
