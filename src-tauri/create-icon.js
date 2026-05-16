const pngToIco = require('png-to-ico');
const fs = require('fs');
const { createCanvas } = require('canvas');

// Create a simple 256x256 PNG
const canvas = createCanvas(256, 256);
const ctx = canvas.getContext('2d');

// Fill with blue
ctx.fillStyle = '#0078D7';
ctx.fillRect(0, 0, 256, 256);

// Add text
ctx.fillStyle = 'white';
ctx.font = 'bold 80px Arial';
ctx.textAlign = 'center';
ctx.textBaseline = 'middle';
ctx.fillText('CD', 128, 128);

// Save as PNG
const buffer = canvas.toBuffer('image/png');
fs.writeFileSync('icons/icon.png', buffer);

// Convert to ICO
pngToIco(buffer).then(buf => {
  fs.writeFileSync('icons/icon.ico', buf);
  console.log('Icon created successfully!');
}).catch(err => {
  console.error('Error:', err);
});
