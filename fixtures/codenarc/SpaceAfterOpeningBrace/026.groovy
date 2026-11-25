
            class MyClass {
                String constructDateRange( Context context ) {
                     def request = context.request
                     switch ( request ) {
                        case DateRangeRequest -> constructReportDate( request.isoStartDate, request.isoStopDate, context.dateFormat )
                        case SingleDateRequest -> constructReportDate( request.isoReportDate, context.dateFormat )
                        default -> null
                    }
                }
            }
        