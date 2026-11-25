
            class MyController extends AbstractController {
                def edit = {
                    if (value != null && value.equalsIgnoreCase(YES)) {
                        editView = "/editCustomMessage"
                    }
                }
            }
        