-- tauri-web 数据库初始化脚本
-- 用法: mysql -uroot -p < sql/init.sql

CREATE DATABASE IF NOT EXISTS `test` DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci;
USE `test`;

CREATE TABLE IF NOT EXISTS `users` (
    `id` bigint unsigned NOT NULL AUTO_INCREMENT COMMENT '自增id',
    `username` varchar(100) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci NOT NULL DEFAULT '' COMMENT '用户名',
    PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- 示例数据（多插几条便于测试分页）
INSERT INTO `users` (`username`) VALUES
('alice'),
('bob'),
('charlie'),
('david'),
('erin'),
('frank'),
('grace'),
('henry'),
('iris'),
('jack'),
('kevin'),
('luna');
