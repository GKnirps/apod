Fetch the NASA astronomical picture of the day
==============================================

This is a small program to fetch the [astronomical picture of the day](https://apod.nasa.gov/apod/astropix.html).

Disclaimer: This program is probably over-engineered yet under-performant and does nothing that a few shell scripts cannot do better. It was only tested on linux and probably only works correctly on unix-like systems that provide a `HOME` variable.

API-Key
-------

This program uses the [APOD-API](https://api.nasa.gov/) to get the high resolution image path and some metadata. It then discards most of the metadata, checks if the current image of the day is actually an image (it may be a video) and downloads the image.

The API requires an API key. This program uses the `DEMO_KEY` API key unless another API key is given. This sets [some limits on the number of requests you can make](https://api.nasa.gov/#signUp). To change that, get yourself an API key from the linked page and enter it in the config file.

Configuration file
------------------

The file `.apod` in the user's home directory is used to configure API key and target directory. If not given, the program uses `DEMO_KEY` as API key and the current directory as target directory.

The `.apod` file is a json file that looks like this:

```
{
  "api_key": "DEMO_KEY",
  "image_dir": "/home/arthurdent/apod_images"
}
```

Possible usages
---------------

I mainly wrote this for myself to get to know rust serde a little better. But also to get nice space pictures.

For example, one could set up a cronjob to load a picture every day and set it as background image. For gnome this could look like this:

```
gsettings set org.gnome.desktop.background picture-uri file://<path-to-image>
```

License
-------

This whole stuff is licensed under the GNU General Public License 3.0
