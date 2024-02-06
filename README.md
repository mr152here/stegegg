# stegegg
Tool to hide and recover messages into/from the images.

### how it works
Stagegg hides each bit of message into random pixels and color channels modifying just an LSB bit of that color. SHA256 hash is calculated from the input key (password) and this value is used to initialize xoshiro256++ PRNG. PRNG creates a sequence of random positions where the message is hidden. Recovering a message from the image is just a reverse process. Without the correct key is almost impossible to guess positions of bits to recover and reconstruct hidden content, even if the "attacker" has access to the original image.

### instalation
clone the git repository (or download the sources) and compile it with cargo.
```
git clone https://github.com/mr152here/stegegg.git
cd stegegg
cargo build --release
cd target/release/
./stegegg -h
```

### usage
To hide "secret message" text into the inputfile.png with "not_very_good_password" as a key:
```
./stegegg -k "not_very_good_password" -m "secret message" inputfile.png outputfile.png
```

File content can be used in the place of key or message:
```
./stegegg -K key.txt -M message.txt inputfile.png outputfile.png
```

To extract message with the key "secret_password":
```
./stegegg -x -k secret_password hidden.png decoded.txt
```

### a few points
- There is no message checksum. If you "recover" something with the incorrect key you will get just random bytes.
- stegegg can read a lot of image formats. However, output makes sense only in lossless formats. So output is limited only to BMP and PNG.
- This is not an encryption. It is recommended to encrypt your message prior to hiding it with the stegegg.

### challenge
A little challenge for determined hackers. Details are hidden in the image. 
![challenge](challenge.png)

 