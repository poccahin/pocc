import blessed from 'blessed';
import contrib from 'blessed-contrib';

// --- 模拟后台数据流 ---
let lifePlusBalance = 10.000000; // 创世注入的 10 USDT 等值 LIFE++
let routedTasks = 0;
let maliciousIntercepted = 0;
let currentNettingPool = 0;

export function launchTerminalDashboard(nodeId: string) {
    const screen = blessed.screen({ smartCSR: true, title: 'Life++ Agent Bank Edge Node' });

    // 1. 创建 12x12 的网格布局
    const grid = new contrib.grid({ rows: 12, cols: 12, screen: screen });

    // 2. 核心模块：右上角的“印钞机” (LIFE++ 余额)
    const balanceBox = grid.set(0, 8, 3, 4, blessed.box, {
        label: ' 💰 LIFE++ EARNINGS (Daily Netting) ',
        content: `\n   ${lifePlusBalance.toFixed(6)} LIFE++\n\n   Status: YIELDING`,
        style: { fg: 'green', border: { fg: 'cyan' } },
        align: 'center'
    });

    // 3. 左上角：节点状态与网络锚定
    const statusBox = grid.set(0, 0, 3, 8, blessed.box, {
        label: ' 🌐 EDGE NODE STATUS ',
        content: `\n  Node ID: ${nodeId}\n  Network: AHIN Global AP2 Mesh\n  L3 Anchor: Solana Mainnet-Beta\n  Role: Micro-Tx Clearing House`,
        style: { fg: 'white', border: { fg: 'cyan' } }
    });

    // 4. 左侧中栏：AP2 路由与防投毒数据统计 (Line Chart)
    const trafficLine = grid.set(3, 0, 4, 8, contrib.line, {
        style: { line: 'yellow', text: 'green', baseline: 'black' },
        xLabelPadding: 3,
        xPadding: 5,
        showLegend: true,
        wholeNumbersOnly: false,
        label: ' ⚡ AP2 Tensor Traffic (Txs/sec) '
    });

    // 5. 右侧中栏：轧账池容量 (Gauge)
    const nettingGauge = grid.set(3, 8, 4, 4, contrib.gauge, {
        label: ' 📦 Zha-Zhang Mempool Fill ',
        stroke: 'blue',
        fill: 'white'
    });

    // 6. 底部面板：实时拦截日志 (Log)
    const logBox = grid.set(7, 0, 5, 12, contrib.log, {
        fg: 'green',
        selectedFg: 'green',
        label: ' 🛡️ PoCC Firewall & Clearing Logs '
    });

    // --- 启动动画与数据模拟引擎 ---

    // 初始化折线图数据
    const trafficData = { title: 'AP2 Routes', x: ['t-5', 't-4', 't-3', 't-2', 't-1', 't0'], y: [0, 0, 0, 0, 0, 0] };

    statusBox.setContent(
        `\n  Node ID: ${nodeId}\n  Network: AHIN Global AP2 Mesh\n  L3 Anchor: Solana Mainnet-Beta\n  Role: Micro-Tx Clearing House\n  Routed: ${routedTasks}\n  Blocked: ${maliciousIntercepted}`
    );

    logBox.log(`[SYS] Node ${nodeId} booted. L0 hardware locks engaged.`);
    logBox.log('[SYS] AP2 Universal Gateway connection established.');

    setInterval(() => {
        // 模拟路由一个合法的张量协作任务
        if (Math.random() > 0.3) {
            routedTasks++;
            const fee = Math.random() * 0.0001;
            currentNettingPool += fee;
            logBox.log(`[AP2] Routed micro-tx from IoT-Sensor to Llama-3-Agent. Fee: +${fee.toFixed(6)} LIFE++`);

            // 模拟动态折线图
            trafficData.y.shift();
            trafficData.y.push(Math.floor(Math.random() * 10) + 5);
            trafficLine.setData([trafficData]);
        }

        // 模拟拦截一次女巫投毒攻击 (10% 概率)
        if (Math.random() > 0.9) {
            maliciousIntercepted++;
            logBox.log('{red-fg}💀 [FATAL] Adversarial Tensor detected! Semantic variance exceeded. Connection dropped.{/red-fg}');
            logBox.log('{yellow-fg}⚖️ [SLASHER] Reputation Slash triggered for attacking IP.{/yellow-fg}');
        }

        // 模拟轧账池满，向 Solana 提交 Merkle Root
        if (currentNettingPool > 0.002) {
            logBox.log('{cyan-fg}🏦 [AGENT BANK] Merkle Root anchored to Solana. Zha-Zhang cleared!{/cyan-fg}');
            lifePlusBalance += currentNettingPool; // 收益落袋
            currentNettingPool = 0; // 清空池子

            balanceBox.setContent(`\n   ${lifePlusBalance.toFixed(6)} LIFE++\n\n   Status: YIELDING`);
            screen.render();
        }

        statusBox.setContent(
            `\n  Node ID: ${nodeId}\n  Network: AHIN Global AP2 Mesh\n  L3 Anchor: Solana Mainnet-Beta\n  Role: Micro-Tx Clearing House\n  Routed: ${routedTasks}\n  Blocked: ${maliciousIntercepted}`
        );

        // 更新 UI 仪表盘
        nettingGauge.setPercent(Math.min((currentNettingPool / 0.002) * 100, 100));
        screen.render();
    }, 800);

    // 退出机制
    screen.key(['escape', 'q', 'C-c'], function () {
        return process.exit(0);
    });

    screen.render();
}
