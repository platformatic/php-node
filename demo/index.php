<?php
if (file_get_contents('php://input') == "Hello, from Node.js!") {
  echo "Hello, from PHP!";
} else {
  echo phpinfo();
}
?>
