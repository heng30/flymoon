- use `init.sh` to replace the `old-project-name`

- Get binary dependencies
```
ldd flymoon | grep -v ld-linux | grep "=> /" | awk '{print $3}' | xargs -n1 basename | cut -d. -f1
```

- inspect deb package
```
dpkg -c flymoon.deb
```
