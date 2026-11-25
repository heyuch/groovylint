
            import com.example.MyService;
            import com.example.test.AbstractTestCase;

            abstract class AbstractMyProfileTestCase extends AbstractTestCase {

                protected void assertEmailAddress(EmailPreference emailPref) {
                    assert emailPref.emailAddress
                }
            }
        