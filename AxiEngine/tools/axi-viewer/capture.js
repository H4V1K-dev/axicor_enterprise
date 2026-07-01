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
    }, { timeout: 10000 });

    // Wait for animations to complete
    await page.waitForTimeout(1500);

    const screenshotPath = path.join(screenshotsDir, outputName);
    console.log(`Taking screenshot: ${screenshotPath}`);
    await page.screenshot({ path: screenshotPath, fullPage: true });
}

async function captureComparison(page, workspaceRoot, datasetNameA, summaryNameA, datasetNameB, summaryNameB, outputName) {
    const datasetDirA = path.join(workspaceRoot, 'artifacts', datasetNameA);
    const summaryFileA = path.join(datasetDirA, summaryNameA);
    const batchesFileA = path.join(datasetDirA, 'node_batches.csv');
    const outputsFileA = path.join(datasetDirA, 'node_outputs.csv');
    const spikesFileA = path.join(datasetDirA, 'node_output_spikes.csv');
    const filesA = [summaryFileA, batchesFileA, outputsFileA, spikesFileA];

    const datasetDirB = path.join(workspaceRoot, 'artifacts', datasetNameB);
    const summaryFileB = path.join(datasetDirB, summaryNameB);
    const batchesFileB = path.join(datasetDirB, 'node_batches.csv');
    const outputsFileB = path.join(datasetDirB, 'node_outputs.csv');
    const spikesFileB = path.join(datasetDirB, 'node_output_spikes.csv');
    const filesB = [summaryFileB, batchesFileB, outputsFileB, spikesFileB];

    const allFiles = [...filesA, ...filesB];
    for (const f of allFiles) {
        if (!fs.existsSync(f)) {
            console.warn(`Warning: File does not exist, skipping comparison: ${f}`);
            return;
        }
    }

    const screenshotsDir = path.join(workspaceRoot, 'artifacts/axi-viewer-screenshots');
    if (!fs.existsSync(screenshotsDir)) {
        fs.mkdirSync(screenshotsDir, { recursive: true });
    }

    console.log(`Loading comparison dataset: A=${datasetNameA}, B=${datasetNameB}...`);
    // Reload the page to clear previous state
    const indexUrl = 'file://' + path.resolve(__dirname, 'index.html');
    await page.goto(indexUrl);

    // Switch to compare mode
    console.log('Switching to Compare mode...');
    await page.click('#btn-compare-mode');

    console.log('Uploading Dataset A (Baseline) telemetry files...');
    const fileInputA = await page.locator('#dirInputA');
    await fileInputA.setInputFiles(filesA);
    await page.evaluate(() => {
        const input = document.getElementById('dirInputA');
        const event = new Event('change', { bubbles: true });
        input.dispatchEvent(event);
    });

    console.log('Uploading Dataset B (Active) telemetry files...');
    const fileInputB = await page.locator('#dirInputB');
    await fileInputB.setInputFiles(filesB);
    await page.evaluate(() => {
        const input = document.getElementById('dirInputB');
        const event = new Event('change', { bubbles: true });
        input.dispatchEvent(event);
    });

    console.log('Waiting for comparison charts to render...');
    await page.waitForFunction(() => {
        const container = document.getElementById('dashboardContainer');
        return container && !container.classList.contains('opacity-40');
    }, { timeout: 10000 });

    // Wait for animations to complete
    await page.waitForTimeout(1500);

    const screenshotPath = path.join(screenshotsDir, outputName);
    console.log(`Taking screenshot: ${screenshotPath}`);
    await page.screenshot({ path: screenshotPath, fullPage: true });
}

async function captureSweep(page, workspaceRoot, outputName) {
    const sweepFile = path.join(workspaceRoot, 'artifacts', 'sweep_summary.csv');
    if (!fs.existsSync(sweepFile)) {
        console.warn(`Warning: File does not exist, skipping sweep capture: ${sweepFile}`);
        return;
    }

    const screenshotsDir = path.join(workspaceRoot, 'artifacts/axi-viewer-screenshots');
    
    console.log('Loading sweep dataset...');
    const indexUrl = 'file://' + path.resolve(__dirname, 'index.html');
    await page.goto(indexUrl);

    console.log('Switching to Parameter Sweep mode...');
    await page.click('#btn-sweep-mode');

    console.log('Uploading sweep_summary.csv...');
    const fileInput = await page.locator('#sweepInput');
    await fileInput.setInputFiles(sweepFile);
    await page.evaluate(() => {
        const input = document.getElementById('sweepInput');
        const event = new Event('change', { bubbles: true });
        input.dispatchEvent(event);
    });

    console.log('Waiting for sweep heatmap to render...');
    await page.waitForFunction(() => {
        const container = document.getElementById('sweepDashboardContainer');
        return container && !container.classList.contains('opacity-40');
    }, { timeout: 10000 });

    // Wait for layouts and visual stability
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
    await page.setViewportSize({ width: 1280, height: 1600 });

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

    // 3. Capture comparison
    await captureComparison(
        page,
        workspaceRoot,
        'local_engine_e2e',
        'local_engine_e2e_summary.json',
        'local_engine_active_e2e',
        'local_engine_active_e2e_summary.json',
        'comparison_e2e_dashboard.png'
    );

    // 4. Capture sweep
    await captureSweep(
        page,
        workspaceRoot,
        'sweep_e2e_dashboard.png'
    );

    console.log('Success! Closing browser.');
    await browser.close();
})();
