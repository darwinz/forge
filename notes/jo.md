---
description: JSON CLI tool (jo) usage
category: General
---
https://github.com/jpmens/jo/blob/master/jo.md

  jo name=Brandon
  {"name": "Brandon"}

  jo tst=1457081292 lat=12.3456 cc=FR name="JP Mens" nada= coffee@T
  {"tst":1457081292,"lat":12.3456,"cc":"FR","name":"JP Mens","nada":null,"coffee":true}

  jo -p -a *
  ["Makefile", "README.md", ...]

  jo -p name=JP object=$(jo fruit=Orange hungry@0 point=$(jo x=10 y=20 list=$(jo -a 1 2 3 4 5))) number=17) sunday@0
  {
    "name": "JP",
    "object": {
      "fruit": "Orange",
      "hungry": false,
      "point": { "x": 10, "y": 20, "list": [1, 2, 3, 4, 5] },
      "number": 17
    },
    "sunday": false
  }
