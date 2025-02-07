# php-stackable

## Build

### Linux

#### Ubuntu

```bash
sudo apt install -y pkg-config build-essential autoconf bison re2c \
                    libxml2-dev libsqlite3-dev
```

#### Fedora

```bash
sudo dnf install -y re2c bison autoconf make libtool ccache \
                    libxml2-devel sqlite-devel
```

### macOS

On macOS we have some extra steps due to homebrew not putting packages in any
standard location.

```bash
brew install autoconf automake bison freetype gettext icu4c krb5 libedit \
                      libiconv libjpeg libpng libxml2 libzip pkg-config \
                      re2c zlib openssl postgresql

export PKG_CONFIG_PATH="$PKG_CONFIG_PATH:$(brew --prefix icu4c)/lib/pkgconfig"
export PKG_CONFIG_PATH="$PKG_CONFIG_PATH:$(brew --prefix krb5)/lib/pkgconfig"
export PKG_CONFIG_PATH="$PKG_CONFIG_PATH:$(brew --prefix libedit)/lib/pkgconfig"
export PKG_CONFIG_PATH="$PKG_CONFIG_PATH:$(brew --prefix libxml2)/lib/pkgconfig"
export PKG_CONFIG_PATH="$PKG_CONFIG_PATH:$(brew --prefix openssl)/lib/pkgconfig"
export PKG_CONFIG_PATH="$PKG_CONFIG_PATH:$(brew --prefix libiconv)/lib/pkgconfig"

export PATH="$(brew --prefix bison)/bin:$PATH"
```
