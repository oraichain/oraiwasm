const blsttcJs = require('./pkg/blsttc_js');

const msg = 'hello';
const sk = Buffer.from(
  'V26g2BlNdF1/uRlqEQmsvIw2tkjYaiB5ey6L+6xrnnE=',
  'base64'
);
const sig =
  'ptNS57WXJoCz8HFyG6EyA73WRkOOlKnf/aB7lJ74K3XH8ZENQI+/3lJqFOLNH8DEBExk0I9WzUWO0hrTB8nakkLDrR92+Wz5Sxl5dFEusujuHOU9cpHWyu3GmflBKKkC';
const ret = Buffer.from(blsttcJs.sign(sk, msg));
console.log(ret.toString('base64') === sig);

const bibars = blsttcJs.generate_bivars(2, 5);
console.log(bibars.get_sum_commit());
for (let i = 0; i < 5; i++) {
  console.log(bibars.get_commit(i), bibars.get_row(i));
}
