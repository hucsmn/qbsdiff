<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE policyconfig PUBLIC "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN"
        "http://www.freedesktop.org/standards/PolicyKit/1/policyconfig.dtd">
<policyconfig>

        <vendor>The systemd Project</vendor>
        <vendor_url>http://www.freedesktop.org/wiki/Software/systemd</vendor_url>

        <action id="org.freedesktop.login1.inhibit-block">
                <description>Allow applications to inhibit system shutdown and suspend</description>
                <message>Authentication is required to allow an application to inhibit system shutdown or suspend.</message>
                <defaults>
                        <allow_any>auth_admin_keep</allow_any>
                        <allow_inactive>yes</allow_inactive>
                        <allow_active>yes</allow_active>
                </defaults>
        </action>

        <action id="org.freedesktop.login1.inhibit-delay">
                <description>Allow applications to delay system shutdown and suspend</description>
                <message>Authentication is required to allow an application to delay system shutdown or suspend.</message>
                <defaults>
                        <allow_any>yes</allow_any>
                        <allow_inactive>yes</allow_inactive>
                        <allow_active>yes</allow_active>
                </defaults>
        </action>

        <action id="org.freedesktop.login1.set-user-linger">
                <description>Allow non-logged-in users to run programs</description>
                <message>Authentication is required to allow a non-logged-in user to run programs.</message>
                <defaults>
                        <allow_any>auth_admin_keep</allow_any>
                        <allow_inactive>auth_admin_keep</allow_inactive>
                        <allow_active>auth_admin_keep</allow_active>
                </defaults>
        </action>

        <action id="org.freedesktop.login1.attach-device">
                <description>Allow attaching devices to seats</description>
                <message>Authentication is required for attaching a device to a seat.</message>
                <defaults>
                        <allow_any>auth_admin_keep</allow_any>
                        <allow_inactive>auth_admin_keep</allow_inactive>
                        <allow_active>auth_admin_keep</allow_active>
                </defaults>
        </action>

        <action id="org.freedesktop.login1.flush-devices">
                <description>Flush device to seat attachments</description>
                <message>Authentication is required for resetting how devices are attached to seats.</message>
                <defaults>
                        <allow_any>auth_admin_keep</allow_any>
                        <allow_inactive>auth_admin_keep</allow_inactive>
                        <allow_active>auth_admin_keep</allow_active>
                </defaults>
        </action>

        <action id="org.freedesktop.login1.power-off">
                <description>Power off the system</description>
                <message>Authentication is required for powering off the system.</message>
                <defaults>
                        <allow_any>auth_admin_keep</allow_any>
                        <allow_inactive>auth_admin_keep</allow_inactive>
                        <allow_active>yes</allow_active>
                </defaults>
        </action>

        <action id="org.freedesktop.login1.power-off-multiple-sessions">
                <description>Power off the system while other users are logged in</description>
                <message>Authentication is required for powering off the system while other users are logged in.</message>
                <defaults>
                        <allow_any>auth_admin_keep</allow_any>
                        <allow_inactive>auth_admin_keep</allow_inactive>
                        <allow_active>auth_admin_keep</allow_active>
                </defaults>
        </action>

        <action id="org.freedesktop.login1.power-off-ignore-inhibit">
                <description>Power off the system while an application asked to inhibit it</description>
                <message>Authentication is required for powering off the system while an application asked to inhibit it.</message>
                <defaults>
                        <allow_any>auth_admin_keep</allow_any>
                        <allow_inactive>auth_admin_keep</allow_inactive>
                        <allow_active>auth_admin_keep</allow_active>
                </defaults>
        </action>

        <action id="org.freedesktop.login1.reboot">
                <description>Reboot the system</description>
                <message>Authentication is required for rebooting the system.</message>
                <defaults>
                        <allow_any>auth_admin_keep</allow_any>
                        <allow_inactive>auth_admin_keep</allow_inactive>
                        <allow_active>yes</allow_active>
                </defaults>
        </action>

        <action id="org.freedesktop.login1.reboot-multiple-sessions">
                <description>Reboot the system while other users are logged in</description>
                <message>Authentication is required for rebooting the system while other users are logged in.</message>
                <defaults>
                        <allow_any>auth_admin_keep</allow_any>
                        <allow_inactive>auth_admin_keep</allow_inactive>
                        <allow_active>auth_admin_keep</allow_active>
                </defaults>
        </action>

        <action id="org.freedesktop.login1.reboot-ignore-inhibit">
                <description>Reboot the system while an application asked to inhibit it</description>
                <message>Authentication is required for rebooting the system while an application asked to inhibit it.</message>
                <defaults>
                        <allow_any>auth_admin_keep</allow_any>
                        <allow_inactive>auth_admin_keep</allow_inactive>
                        <allow_active>auth_admin_keep</allow_active>
                </defaults>
        </action>

        <action id="org.freedesktop.login1.suspend">
                <description>Suspend the system</description>
                <message>Authentication is required for suspending the system.</message>
                <defaults>
                        <allow_any>auth_admin_keep</allow_any>
                        <allow_inactive>auth_admin_keep</allow_inactive>
                        <allow_active>yes</allow_active>
                </defaults>
        </action>

        <action id="org.freedesktop.login1.suspend-multiple-sessions">
                <description>Suspend the system while other users are logged in</description>
                <message>Authentication is required for suspending the system while other users are logged in.</message>
                <defaults>
                        <allow_any>auth_admin_keep</allow_any>
                        <allow_inactive>auth_admin_keep</allow_inactive>
                        <allow_active>yes</allow_active>
                </defaults>
        </action>

        <action id="org.freedesktop.login1.suspend-ignore-inhibit">
                <description>Suspend the system while an application asked to inhibit it</description>
                <message>Authentication is required for suspending the system while an application asked to inhibit it.</message>
                <defaults>
                        <allow_any>auth_admin_keep</allow_any>
                        <allow_inactive>auth_admin_keep</allow_inactive>
                        <allow_active>auth_admin_keep</allow_active>
                </defaults>
        </action>

        <action id="org.freedesktop.login1.hibernate">
                <description>Hibernate the system</description>
                <message>Authentication is required for hibernating the system.</message>
                <defaults>
                        <allow_any>auth_admin_keep</allow_any>
                        <allow_inactive>auth_admin_keep</allow_inactive>
                        <allow_active>yes</allow_active>
                </defaults>
        </action>

        <action id="org.freedesktop.login1.hibernate-multiple-sessions">
                <description>Hibernate the system while other users are logged in</description>
                <message>Authentication is required for hibernating the system while other users are logged in.</message>
                <defaults>
                        <allow_any>auth_admin_keep</allow_any>
                        <allow_inactive>auth_admin_keep</allow_inactive>
                        <allow_active>auth_admin_keep</allow_active>
                </defaults>
        </action>

        <action id="org.freedesktop.login1.hibernate-ignore-inhibit">
                <description>Hibernate the system while an application asked to inhibit it</description>
                <message>Authentication is required for hibernating the system while an application asked to inhibit it.</message>
                <defaults>
                        <allow_any>auth_admin_keep</allow_any>
                        <allow_inactive>auth_admin_keep</allow_inactive>
                        <allow_active>auth_admin_keep</allow_active>
                </defaults>
        </action>

</policyconfig>