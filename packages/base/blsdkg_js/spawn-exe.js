const exec = require('child_process').execFile;

let array = [
    'dev1',
    'dev2',
    'dev3',
    'dev4',
    'dev5',
    'test1',
    'test2',
    'test3',
    'test4',
    'test5'
];

for (let i = 0; i < 10; i++) {
    let fileName = './vrf-runner-linux';
    // if (i > 2) {
    //     fileName = 'index-error.js'
    // }
    const ls = exec(fileName, {
        env: Object.assign(process.env, { NODE_ENV: array[i], TESTNET: true }),
        cwd: process.cwd()
    });

    ls.stdout.on('data', (data) => {
        console.log(`${array[i]} stdout: ${data}`);
    });
    ls.stderr.on('data', (data) => {
        console.error(`stderr: ${data}`);
    });
}
