// MongoDB 初始化脚本
// 微博扫码登录桌面应用数据库初始化
//
// 说明:
//   1. 此脚本在容器首次启动时自动执行
//   2. 数据库 weibo_desktop 已由环境变量 MONGO_INITDB_DATABASE 指定

// ==========================================
// 1. 切换到应用数据库
// ==========================================
db = db.getSiblingDB('weibo_desktop');

print('==========================================');
print('MongoDB 初始化脚本开始执行');
print('数据库: weibo_desktop');
print('==========================================');

// ==========================================
// 2. 创建集合
// ==========================================

// 用户活动日志集合
db.createCollection('activity_logs', {
  validator: {
    $jsonSchema: {
      bsonType: 'object',
      required: ['account_id', 'action', 'timestamp'],
      properties: {
        account_id: {
          bsonType: 'string',
          description: '用户账户ID (对应PostgreSQL的accounts.id)'
        },
        action: {
          bsonType: 'string',
          enum: ['login', 'logout', 'qrcode_generate', 'qrcode_scan', 'cookies_refresh', 'cookies_validate'],
          description: '操作类型'
        },
        timestamp: {
          bsonType: 'date',
          description: '操作时间'
        },
        metadata: {
          bsonType: 'object',
          description: '额外的元数据'
        },
        ip_address: {
          bsonType: 'string',
          description: 'IP地址'
        },
        user_agent: {
          bsonType: 'string',
          description: '用户代理'
        }
      }
    }
  },
  validationLevel: 'moderate',
  validationAction: 'warn'
});

// 二维码记录集合
db.createCollection('qrcode_records', {
  validator: {
    $jsonSchema: {
      bsonType: 'object',
      required: ['qrcode_id', 'status', 'created_at'],
      properties: {
        qrcode_id: {
          bsonType: 'string',
          description: '二维码ID'
        },
        qrcode_url: {
          bsonType: 'string',
          description: '二维码URL'
        },
        status: {
          bsonType: 'string',
          enum: ['pending', 'scanned', 'confirmed', 'expired', 'cancelled'],
          description: '二维码状态'
        },
        created_at: {
          bsonType: 'date',
          description: '创建时间'
        },
        expires_at: {
          bsonType: 'date',
          description: '过期时间'
        },
        scanned_at: {
          bsonType: 'date',
          description: '扫描时间'
        },
        confirmed_at: {
          bsonType: 'date',
          description: '确认时间'
        },
        account_id: {
          bsonType: 'string',
          description: '关联的账户ID'
        }
      }
    }
  },
  validationLevel: 'moderate',
  validationAction: 'warn'
});

// 系统事件集合 (用于审计和监控)
db.createCollection('system_events', {
  validator: {
    $jsonSchema: {
      bsonType: 'object',
      required: ['event_type', 'timestamp'],
      properties: {
        event_type: {
          bsonType: 'string',
          description: '事件类型'
        },
        timestamp: {
          bsonType: 'date',
          description: '事件时间'
        },
        severity: {
          bsonType: 'string',
          enum: ['info', 'warning', 'error', 'critical'],
          description: '严重级别'
        },
        message: {
          bsonType: 'string',
          description: '事件消息'
        },
        details: {
          bsonType: 'object',
          description: '事件详情'
        },
        source: {
          bsonType: 'string',
          description: '事件来源 (tauri-backend, playwright, etc.)'
        }
      }
    }
  },
  validationLevel: 'moderate',
  validationAction: 'warn'
});

// ==========================================
// 3. 创建索引
// ==========================================

// activity_logs 索引
db.activity_logs.createIndex({ account_id: 1, timestamp: -1 });
db.activity_logs.createIndex({ action: 1, timestamp: -1 });
db.activity_logs.createIndex({ timestamp: -1 });
db.activity_logs.createIndex({ ip_address: 1 });

// qrcode_records 索引
db.qrcode_records.createIndex({ qrcode_id: 1 }, { unique: true });
db.qrcode_records.createIndex({ status: 1, created_at: -1 });
db.qrcode_records.createIndex({ account_id: 1, created_at: -1 });
db.qrcode_records.createIndex({ expires_at: 1 });

// system_events 索引
db.system_events.createIndex({ event_type: 1, timestamp: -1 });
db.system_events.createIndex({ severity: 1, timestamp: -1 });
db.system_events.createIndex({ timestamp: -1 });
db.system_events.createIndex({ source: 1, timestamp: -1 });

// ==========================================
// 4. 设置 TTL 索引 (自动清理过期数据)
// ==========================================

// 活动日志保留90天
db.activity_logs.createIndex(
  { timestamp: 1 },
  { expireAfterSeconds: 90 * 24 * 60 * 60 }
);

// 二维码记录保留7天
db.qrcode_records.createIndex(
  { created_at: 1 },
  { expireAfterSeconds: 7 * 24 * 60 * 60 }
);

// 系统事件保留30天
db.system_events.createIndex(
  { timestamp: 1 },
  { expireAfterSeconds: 30 * 24 * 60 * 60 }
);

// ==========================================
// 5. 插入初始数据
// ==========================================

// 插入初始系统事件
db.system_events.insertOne({
  event_type: 'database_initialized',
  timestamp: new Date(),
  severity: 'info',
  message: 'MongoDB 数据库初始化完成',
  details: {
    database: 'weibo_desktop',
    collections: db.getCollectionNames(),
    version: '1.0.0'
  },
  source: 'init-script'
});

// ==========================================
// 6. 创建视图 (可选)
// ==========================================

// 最近登录活动视图
db.createView(
  'recent_login_activities',
  'activity_logs',
  [
    {
      $match: {
        action: { $in: ['login', 'logout'] }
      }
    },
    {
      $sort: { timestamp: -1 }
    },
    {
      $limit: 1000
    },
    {
      $project: {
        account_id: 1,
        action: 1,
        timestamp: 1,
        ip_address: 1,
        user_agent: 1
      }
    }
  ]
);

// ==========================================
// 7. 输出初始化信息
// ==========================================
print('==========================================');
print('集合创建完成:');
db.getCollectionNames().forEach(function(collection) {
  print('  - ' + collection);
});
print('');
print('索引创建完成');
print('TTL索引配置完成 (自动清理过期数据)');
print('');
print('MongoDB 数据库初始化完成!');
print('==========================================');
