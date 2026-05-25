const pngToIco = require('png-to-ico').default;
const fs = require('fs');
const path = require('path');

async function createIco() {
  const pngPath = path.join(__dirname, 'src-tauri/icons/icon.png');
  const icoPath = path.join(__dirname, 'src-tauri/icons/icon.ico');
  
  try {
    console.log('Converting PNG to ICO...');
    const pngBuffer = fs.readFileSync(pngPath);
    const icoBuffer = await pngToIco(pngBuffer);
    fs.writeFileSync(icoPath, icoBuffer);
    console.log('✓ icon.ico created successfully!');
  } catch (error) {
    console.error('Error:', error);
    process.exit(1);
  }
}

createIco();
