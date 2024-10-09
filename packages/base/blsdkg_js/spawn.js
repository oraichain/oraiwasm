const { spawn } = require('child_process');

let array = [
    'dev1',
    'dev2',
];

for (let i = 0; i < 3; i++) {
    let fileName = 'index-test.js';
    // if (i > 2) {
    //     fileName = 'index-error.js'
    // }
    const ls = spawn('node', [fileName], {
        env: Object.assign(process.env, { NODE_ENV: array[i] }),
        cwd: process.cwd()
    });

    ls.stdout.on('data', (data) => {
        console.log(`${array[i]} stdout: ${data}`);
    });
    ls.stderr.on('data', (data) => {
        console.error(`stderr: ${data}`);
    });
}
