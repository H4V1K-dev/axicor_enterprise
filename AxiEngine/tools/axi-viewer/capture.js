const { chromium } = require('playwright');
const path = require('path');
const fs = require('fs');

async function captureDataset(page, workspaceRoot, datasetName, summaryName, outputName) {
    const datasetDir = path.join(workspaceRoot, 'artifacts', datasetName);
    const summaryFile = path.join(datasetDir, summaryName);
    const batchesFile = path.join(datasetDir, 'node_batches.csv');
    const outputsFile = path.join(datasetDir, 'node_outputs.csv');
    const spikesFile = path.join(datasetDir, 'node_output_spikes.csv');

    const files = [summaryFile, batchesFile, outputsFile, spikesFile];
    for (const f of files) {
        if (!fs.existsSync(f)) {
            console.warn(`Warning: File does not exist, skipping this dataset: ${f}`);
            return;
        }
    }

    const screenshotsDir = path.join(workspaceRoot, 'artifacts/axi-viewer-screenshots');
    if (!fs.existsSync(screenshotsDir)) {
        fs.mkdirSync(screenshotsDir, { recursive: true });
    }

    console.log(`Loading dataset: ${datasetName}...`);
    // Reload the page to clear previous state
    const indexUrl = 'file://' + path.resolve(__dirname, 'index.html');
    await page.goto(indexUrl);

    console.log('Uploading telemetry files...');
    const fileInput = await page.locator('#dirInput');
    await fileInput.setInputFiles(files);
    
    // Manually dispatch change event to ensure handler fires
    await page.evaluate(() => {
        const input = document.getElementById('dirInput');
        const event = new Event('change', { bubbles: true });
        input.dispatchEvent(event);
    });

    console.log('Waiting for charts to render...');
    await page.waitForFunction(() => {
        const container = document.getElementById('dashboardContainer');
        return container && !container.classList.contains('opacity-40');
    }, { timeout: 5000 });

    // Wait for animations to complete
    await page.waitForTimeout(1500);

    const screenshotPath = path.join(screenshotsDir, outputName);
    console.log(`Taking screenshot: ${screenshotPath}`);
    await page.screenshot({ path: screenshotPath, fullPage: true });
}

(async () => {
    const workspaceRoot = path.resolve(__dirname, '../..');
    
    console.log('Launching browser...');
    const browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    
    // Redirect browser console logs and errors to node console
    page.on('console', msg => console.log('BROWSER LOG:', msg.text()));
    page.on('pageerror', err => console.error('BROWSER ERROR:', err));
    
    // Set large viewport
    await page.setViewportSize({ width: 1280, height: 1000 });

    // 1. Capture baseline
    await captureDataset(
        page, 
        workspaceRoot, 
        'local_engine_e2e', 
        'local_engine_e2e_summary.json', 
        'baseline_e2e_dashboard.png'
    );

    // 2. Capture active
    await captureDataset(
        page, 
        workspaceRoot, 
        'local_engine_active_e2e', 
        'local_engine_active_e2e_summary.json', 
        'active_e2e_dashboard.png'
    );

    console.log('Success! Closing browser.');
    await browser.close();
})();
