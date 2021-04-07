var express = require('express');
var app = express();

//Allow all requests from all domains & localhost
app.all('/*', function (req, res, next) {
  res.header('Access-Control-Allow-Origin', '*');
  res.header(
    'Access-Control-Allow-Headers',
    'X-Requested-With, Content-Type, Accept'
  );
  res.header('Access-Control-Allow-Methods', 'POST, GET');
  next();
});

app.use(express.json());
app.use(express.urlencoded({ extended: false }));

app.post('/', function (req, res) {
  const { name } = req.body;
  res.status(200).json({ message: `Hello ${name}` });
});
const port = process.env.PORT || 6069;
console.log(`Server is listening at ${port}!`);
app.listen(port);
