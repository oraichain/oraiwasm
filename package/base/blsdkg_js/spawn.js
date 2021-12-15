const { spawn } = require('child_process');

let array = [
    'test1',
    'test2',
    'test3'
];

for (let i = 0; i < 10; i++) {
    let fileName = 'index.js';
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
