# Face detection model

`maskcam` uses the YuNet face detector distributed by the OpenCV Zoo project.
Download the model with:

```sh
./scripts/download-model.sh
```

The downloaded ONNX file is intentionally ignored by Git. The model is licensed
under the MIT License; see the upstream model directory for its full license:
<https://github.com/opencv/opencv_zoo/tree/main/models/face_detection_yunet>.

