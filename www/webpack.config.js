const CopyPlugin = require("copy-webpack-plugin");
const path = require('path');

module.exports = {
  entry: "./bootstrap.js",
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "bootstrap.js",
  },
  mode: "production",
  plugins: [
    new CopyPlugin({
      patterns: [
        { from: "index.html", to: "index.html" },
        { from: "style.css", to: "style.css" },
      ],
    }),
  ],
  module: {
    rules: [
      {
        test: /\.wasm$/,
        type: "asset/inline",
      },
    ],
  },
};
