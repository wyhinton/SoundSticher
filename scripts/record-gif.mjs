import inquirer from 'inquirer';
import { execa } from 'execa';
import path from 'path';

const DEFAULT_DELAY = 2; // seconds
const DEFAULT_DURATION = 5; // seconds
const DEFAULT_REGION = '0,0,800,600'; // x,y,width,height
const DEFAULT_FPS = '15fps';

async function main() {
  // Parse command-line arguments
  const args = process.argv.slice(2);
  const argMap = {};

  // Parse --key=value arguments
  args.forEach(arg => {
    const match = arg.match(/^--([^=]+)=(.+)$/);
    if (match) {
      argMap[match[1]] = match[2];
    }
  });

  // Check if we have command-line arguments or use environment variables
  const usePrompts = args.length === 0 && !process.env.RECORD_REGION;

  let answers;

  if (usePrompts) {
    // Interactive mode - use inquirer prompts
    answers = await inquirer.prompt([
      { name: 'name', message: 'What are you recording?', default: 'recording' },
      {
        name: 'delay',
        message: `Delay before recording (seconds)`,
        default: DEFAULT_DELAY,
        type: 'number',
      },
      {
        name: 'duration',
        message: `Recording duration (seconds)`,
        default: DEFAULT_DURATION,
        type: 'number',
      },
      { name: 'region', message: `Capture region (x,y,w,h)`, default: DEFAULT_REGION },
    ]);
  } else {
    // Non-interactive mode - use command-line args or env vars
    answers = {
      name: argMap.name || 'recording',
      delay: parseInt(argMap.delay || process.env.RECORD_DELAY || DEFAULT_DELAY),
      duration: parseInt(argMap.duration || process.env.RECORD_DURATION || DEFAULT_DURATION),
      region: argMap.region || process.env.RECORD_REGION || DEFAULT_REGION,
    };

    console.log('🎬 Recording with parameters:');
    console.log(`   Name: ${answers.name}`);
    console.log(`   Delay: ${answers.delay}s`);
    console.log(`   Duration: ${answers.duration}s`);
    console.log(`   Region: ${answers.region}`);
  }

  const name = answers.name;
  const delay = answers.delay ?? DEFAULT_DELAY;
  const duration = answers.duration ?? DEFAULT_DURATION;
  const region = answers.region ?? DEFAULT_REGION;

  const outputPath = path.resolve('./gif_output', `${name}.gif`);

  console.log(`\n⏱ Starting in ${delay}s for ${duration}s...`);

  await new Promise(r => setTimeout(r, delay * 1000));

  console.log(`\n🎬 Recording: ${name}.gif`);

  // Construct time limit in hh:mm:ss format
  const hh = String(Math.floor(duration / 3600)).padStart(2, '0');
  const mm = String(Math.floor((duration % 3600) / 60)).padStart(2, '0');
  const ss = String(duration % 60).padStart(2, '0');
  const timespan = `${hh}:${mm}:${ss}`;

  try {
    await execa(
      'C:\\Program Files (x86)\\ScreenToGif\\ScreenToGif.exe',
      [
        '-n', // New instance
        '-o',
        's', // Open screen recorder
        '-r',
        region, // Region
        '-f',
        DEFAULT_FPS, // FPS
        '-l',
        timespan, // Duration
        '-c', // Start capture immediately
        '-save',
        outputPath, // Save to specific path (ScreenToGif >= v2.38 supports this)
      ],
      { stdio: 'inherit' }
    );

    console.log(`\n✅ Done: ${outputPath}`);
  } catch (err) {
    console.error('❌ Recording failed:');
    console.error(err);
  } finally {
  }
}

main();
