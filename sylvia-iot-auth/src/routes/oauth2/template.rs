pub const LOGIN: &'static str = "\
<!DOCTYPE html>\
<html>\
<head>\
<meta charset=\"utf-8\">\
<meta http-equiv=\"X-UA-Compatible\" content=\"IE=edge\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<title>Log in</title>\
<link rel=\"stylesheet\" href=\"https://stackpath.bootstrapcdn.com/bootstrap/3.4.1/css/bootstrap.min.css\">\
<script src=\"https://code.jquery.com/jquery-3.7.1.min.js\"></script>\
<script src=\"https://stackpath.bootstrapcdn.com/bootstrap/3.4.1/js/bootstrap.min.js\"></script>\
<style>\
    .login-form {\
        width: 340px;\
        margin: 50px auto;\
    }\
    .login-form form {\
        margin-bottom: 15px;\
        background: #f7f7f7;\
        box-shadow: 0px 2px 2px rgba(0, 0, 0, 0.3);\
        padding: 30px;\
    }\
    .login-form h2 {\
        margin: 0 0 15px;\
    }\
    .form-control, .btn {\
        min-height: 38px;\
        border-radius: 2px;\
    }\
    .btn {\
        font-size: 15px;\
        font-weight: bold;\
    }\
</style>\
</head>\
<body>\
<div class=\"login-form\">\
    <form action=\"{{scope_path}}/oauth2/login\" method=\"post\">\
        <h2 class=\"text-center\">Log in</h2>\
        <input type=\"hidden\" name=\"state\" value=\"{{state}}\">\
        <div class=\"form-group\">\
            <input type=\"text\" class=\"form-control\" name=\"account\" placeholder=\"Username\" required autofocus>\
        </div>\
        <div class=\"form-group\">\
            <input type=\"password\" class=\"form-control\" name=\"password\" placeholder=\"Password\" required>\
        </div>\
        <div class=\"form-group\">\
            <button type=\"submit\" class=\"btn btn-primary btn-block\">Log in</button>\
        </div>\
    </form>\
</div>\
</body>\
</html>";

pub const GRANT: &'static str= "\
<!DOCTYPE html>\
<html>\
<head>\
<meta charset=\"utf-8\">\
<meta http-equiv=\"X-UA-Compatible\" content=\"IE=edge\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<title>Grant</title>\
<link rel=\"stylesheet\" href=\"https://stackpath.bootstrapcdn.com/bootstrap/3.4.1/css/bootstrap.min.css\">\
<script src=\"https://code.jquery.com/jquery-3.7.1.min.js\"></script>\
<script src=\"https://stackpath.bootstrapcdn.com/bootstrap/3.4.1/js/bootstrap.min.js\"></script>\
<style>\
    .grant-form {\
        width: 340px;\
        margin: 50px auto;\
    }\
    .grant-form form {\
        margin-bottom: 15px;\
        background: #f7f7f7;\
        box-shadow: 0px 2px 2px rgba(0, 0, 0, 0.3);\
        padding: 30px;\
    }\
    .grant-form h2 {\
        margin: 0 0 15px;\
    }\
    .grant-control, .btn {\
        min-height: 38px;\
        border-radius: 2px;\
    }\
    .btn {\
        font-size: 15px;\
        font-weight: bold;\
    }\
</style>\
</head>\
<body>\
<div class=\"grant-form\">\
    <form class=\"form-inline\" action=\"{{scope_path}}/oauth2/authorize\" method=\"post\">\
        <h2 class=\"text-center\">Grant {{ client_name }}</h2>\
        <input type=\"hidden\" id=\"allow\" name=\"allow\">\
        <input type=\"hidden\" name=\"session_id\" value=\"{{session_id}}\">\
        <input type=\"hidden\" name=\"client_id\" value=\"{{client_id}}\">\
        <input type=\"hidden\" name=\"response_type\" value=\"{{response_type}}\">\
        <input type=\"hidden\" name=\"redirect_uri\" value=\"{{redirect_uri}}\">\
        {% if scope %}
            <input type=\"hidden\" name=\"scope\" value=\"{{scope}}\">\
        {% endif %}
        {% if state %}
            <input type=\"hidden\" name=\"state\" value=\"{{state}}\">\
        {% endif %}
        <div class=\"row\">\
            <button type=\"submit\" class=\"btn btn-primary btn-block\" onclick=\"document.getElementById('allow').value='{{allow_value}}';\">Accept</button>\
            <button type=\"submit\" class=\"btn btn-block\" onclick=\"document.getElementById('allow').value='no';\">Deny</button>\
        </div>\
    </form>\
</div>\
</body>\
</html>";
